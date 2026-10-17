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
    private var explicitStop = false
    private val progress = object : Runnable {
        override fun run() {
            if (player.isPlaying) {
                AudioPlugin.sendEvent("progress", player.currentPosition.coerceAtLeast(0), duration())
            }
            handler.postDelayed(this, 500)
        }
    }

    override fun onCreate() {
        super.onCreate()
        player = ExoPlayer.Builder(this).build().apply {
            setAudioAttributes(AudioAttributes.DEFAULT, true)
            setHandleAudioBecomingNoisy(true)
            addListener(object : Player.Listener {
                override fun onPlaybackStateChanged(state: Int) {
                    if (state == Player.STATE_READY) loading = false
                    if (state == Player.STATE_ENDED && !explicitStop) {
                        AudioPlugin.sendEvent("ended", currentPosition.coerceAtLeast(0), duration())
                    } else {
                        publishState()
                    }
                }
                override fun onIsPlayingChanged(isPlaying: Boolean) {
                    if (player.playbackState != Player.STATE_ENDED) publishState()
                }
                override fun onPlayerError(error: androidx.media3.common.PlaybackException) {
                    loading = false
                    AudioPlugin.sendError(error.message ?: "Falha ao reproduzir áudio")
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

    fun load(uri: android.net.Uri, headers: Map<String, String>, stream: Boolean) {
        explicitStop = true
        player.stop()
        explicitStop = false
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
    fun stopPlayback() {
        explicitStop = true
        player.stop()
        player.clearMediaItems()
        explicitStop = false
        publishState()
    }
    fun seek(positionMs: Long) { player.seekTo(positionMs); publishState() }
    fun volume(value: Float) { player.volume = value }

    private fun duration(): Long = player.duration.takeIf { it != C.TIME_UNSET && it >= 0 } ?: 0L
    private fun publishState() {
        val status = when {
            player.mediaItemCount == 0 -> "stopped"
            loading -> "loading"
            player.isPlaying -> "playing"
            else -> "paused"
        }
        AudioPlugin.sendEvent("state", player.currentPosition.coerceAtLeast(0), duration(), status)
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
                AudioPlugin.sendError(error.message ?: "Falha no serviço de áudio")
            }
        }
    }
}
