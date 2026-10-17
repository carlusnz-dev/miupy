package dev.miupy.app

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class AudioArgs {
    var path: String? = null
    var url: String? = null
    var headers: Map<String, String> = emptyMap()
    var positionMs: Long = 0
    var volume: Float? = null
    var generation: Long = 0
}

@InvokeArg
class ListenArgs { lateinit var channel: Channel }

// @req RF-44 RF-18 RF-02 RF-28
@TauriPlugin
class AudioPlugin(private val activity: Activity) : Plugin(activity) {
    private val main = Handler(Looper.getMainLooper())

    @Command
    fun listen(invoke: Invoke) {
        channel = invoke.parseArgs(ListenArgs::class.java).channel
        invoke.resolve()
    }

    private fun execute(invoke: Invoke, action: (PlaybackService) -> Unit) {
        main.post {
            val service = PlaybackService.instance
            if (service != null) {
                PlaybackService.run(service, action)
            } else {
                // O startService só cria o serviço depois, no looper: a ação espera o onCreate.
                PlaybackService.pending.add(action)
                try {
                    val context = activity.applicationContext
                    context.startService(Intent(context, PlaybackService::class.java))
                } catch (error: IllegalStateException) {
                    // App em segundo plano no Android 8+: não dá para subir o serviço agora.
                    PlaybackService.pending.clear()
                    sendError("Não deu para iniciar o áudio em segundo plano", 0)
                }
            }
        }
        invoke.resolve()
    }

    @Command
    fun load(invoke: Invoke) {
        val args = invoke.parseArgs(AudioArgs::class.java)
        val path = args.path ?: return invoke.reject("Caminho ausente")
        execute(invoke) { it.load(Uri.fromFile(java.io.File(path)), emptyMap(), false, args.generation) }
    }

    @Command
    fun loadUrl(invoke: Invoke) {
        val args = invoke.parseArgs(AudioArgs::class.java)
        val url = args.url ?: return invoke.reject("URL ausente")
        execute(invoke) { it.load(Uri.parse(url), args.headers, true, args.generation) }
    }

    @Command fun play(invoke: Invoke) = execute(invoke) { it.play(); requestNotificationPermission() }
    @Command fun pause(invoke: Invoke) = execute(invoke) { it.pause() }
    @Command fun stop(invoke: Invoke) {
        val generation = invoke.parseArgs(AudioArgs::class.java).generation
        execute(invoke) { it.stopPlayback(generation) }
    }
    @Command fun seek(invoke: Invoke) {
        val position = invoke.parseArgs(AudioArgs::class.java).positionMs
        execute(invoke) { it.seek(position) }
    }
    @Command fun volume(invoke: Invoke) {
        val value = invoke.parseArgs(AudioArgs::class.java).volume ?: return invoke.reject("Volume ausente")
        execute(invoke) { it.volume(value) }
    }

    private fun requestNotificationPermission() {
        if (Build.VERSION.SDK_INT >= 33 &&
            activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED &&
            !permissionRequested) {
            permissionRequested = true
            activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 4401)
        }
    }

    companion object {
        @Volatile private var channel: Channel? = null
        /** Último volume efetivo pedido; reaplicado se o sistema recriar o serviço. */
        @Volatile var lastVolume = 1f
        private var permissionRequested = false

        fun sendEvent(kind: String, positionMs: Long, durationMs: Long, generation: Long, status: String? = null) {
            val data = JSObject().apply {
                put("kind", kind)
                put("generation", generation)
                put("positionMs", positionMs)
                put("durationMs", durationMs)
                if (status != null) put("status", status)
            }
            channel?.send(data)
        }

        fun sendError(message: String, generation: Long) {
            channel?.send(JSObject().apply {
                put("kind", "error")
                put("generation", generation)
                put("message", message)
            })
        }
    }
}
