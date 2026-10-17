package dev.miupy.app

import android.content.Intent
import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

// @req RF-44 RF-01
class PlaybackService : MediaSessionService() {
    private lateinit var player: ExoPlayer
    private lateinit var session: MediaSession
    private val handler = Handler(Looper.getMainLooper())
    private var loading = false
    /** Geração do último load/stop vinda do Rust; carimba os eventos para o Rust descartar os atrasados. */
    var generation = 0L
        private set
    private val progress = object : Runnable {
        override fun run() {
            if (player.isPlaying) {
                AudioPlugin.sendEvent("progress", player.currentPosition.coerceAtLeast(0), duration(), generation)
            }
            handler.postDelayed(this, 500)
        }
    }

    override fun onCreate() {
        super.onCreate()
        player = ExoPlayer.Builder(this).build().apply {
            setAudioAttributes(
                AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
                true,
            )
            setHandleAudioBecomingNoisy(true)
            // Tela apagada: mantém CPU e Wi-Fi acordados enquanto toca (local e streaming).
            setWakeMode(C.WAKE_MODE_NETWORK)
            // Serviço recriado (o sistema o matou): volta com o último volume pedido pelo Rust.
            volume = AudioPlugin.lastVolume
            addListener(object : Player.Listener {
                override fun onPlaybackStateChanged(state: Int) {
                    if (state == Player.STATE_READY) loading = false
                    if (state == Player.STATE_ENDED) {
                        AudioPlugin.sendEvent("ended", currentPosition.coerceAtLeast(0), duration(), generation)
                    } else {
                        publishState()
                    }
                }
                override fun onIsPlayingChanged(isPlaying: Boolean) {
                    if (player.playbackState != Player.STATE_ENDED) publishState()
                }
                override fun onPlayerError(error: androidx.media3.common.PlaybackException) {
                    loading = false
                    AudioPlugin.sendError(error.message ?: "Falha ao reproduzir áudio", generation)
                }
            })
        }
        session = MediaSession.Builder(this, player).build()
        instance = this
        pending.forEach { run(this, it) }
        pending.clear()
        handler.post(progress)
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession = session

    fun load(uri: android.net.Uri, headers: Map<String, String>, stream: Boolean, generation: Long) {
        // A geração muda depois do stop(): o estado que ele publica é da faixa anterior.
        player.stop()
        this.generation = generation
        // O stop() não zera playWhenReady: sem isto a faixa nova tocaria sozinha (o contrato é pausada).
        player.playWhenReady = false
        val item = androidx.media3.common.MediaItem.fromUri(uri)
        if (stream) {
            val factory = androidx.media3.datasource.DefaultHttpDataSource.Factory()
                .setDefaultRequestProperties(headers)
            player.setMediaSource(androidx.media3.exoplayer.source.ProgressiveMediaSource.Factory(factory).createMediaSource(item))
        } else {
            player.setMediaItem(item)
        }
        loading = stream
        player.prepare()
        publishState()
    }

    fun play() { player.play(); publishState() }
    fun pause() { player.pause(); publishState() }
    fun stopPlayback(generation: Long) {
        this.generation = generation
        player.stop()
        player.clearMediaItems()
        publishState()
    }
    fun seek(positionMs: Long) { player.seekTo(positionMs); publishState() }
    fun volume(value: Float) { AudioPlugin.lastVolume = value; player.volume = value }

    private fun duration(): Long = player.duration.takeIf { it != C.TIME_UNSET && it >= 0 } ?: 0L
    private fun publishState() {
        val status = when {
            player.mediaItemCount == 0 -> "stopped"
            loading -> "loading"
            player.isPlaying -> "playing"
            else -> "paused"
        }
        AudioPlugin.sendEvent("state", player.currentPosition.coerceAtLeast(0), duration(), generation, status)
    }

    override fun onTaskRemoved(rootIntent: Intent?) {
        if (!player.isPlaying) stopSelf()
    }

    override fun onDestroy() {
        handler.removeCallbacks(progress)
        instance = null
        session.release()
        player.release()
        super.onDestroy()
    }

    companion object {
        @Volatile var instance: PlaybackService? = null
            private set
        /** Comandos que chegaram antes do onCreate; só tocados na thread principal. */
        val pending = mutableListOf<(PlaybackService) -> Unit>()

        fun run(service: PlaybackService, action: (PlaybackService) -> Unit) {
            try {
                action(service)
            } catch (error: Exception) {
                AudioPlugin.sendError(error.message ?: "Falha no serviço de áudio", service.generation)
            }
        }
    }
}
