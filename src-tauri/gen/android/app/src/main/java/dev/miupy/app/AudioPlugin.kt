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
    var volume: Float = 1f
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
                activity.startService(Intent(activity, PlaybackService::class.java))
            }
        }
        invoke.resolve()
    }

    @Command
    fun load(invoke: Invoke) {
        val path = invoke.parseArgs(AudioArgs::class.java).path ?: return invoke.reject("Caminho ausente")
        execute(invoke) { it.load(Uri.fromFile(java.io.File(path)), emptyMap(), false) }
    }

    @Command
    fun loadUrl(invoke: Invoke) {
        val args = invoke.parseArgs(AudioArgs::class.java)
        val url = args.url ?: return invoke.reject("URL ausente")
        execute(invoke) { it.load(Uri.parse(url), args.headers, true) }
    }

    @Command fun play(invoke: Invoke) = execute(invoke) { it.play(); requestNotificationPermission() }
    @Command fun pause(invoke: Invoke) = execute(invoke) { it.pause() }
    @Command fun stop(invoke: Invoke) = execute(invoke) { it.stopPlayback() }
    @Command fun seek(invoke: Invoke) {
        val position = invoke.parseArgs(AudioArgs::class.java).positionMs
        execute(invoke) { it.seek(position) }
    }
    @Command fun volume(invoke: Invoke) {
        val value = invoke.parseArgs(AudioArgs::class.java).volume
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
        private var permissionRequested = false

        fun sendEvent(kind: String, positionMs: Long, durationMs: Long, status: String? = null) {
            val data = JSObject().apply {
                put("kind", kind)
                put("positionMs", positionMs)
                put("durationMs", durationMs)
                if (status != null) put("status", status)
            }
            channel?.send(data)
        }

        fun sendError(message: String) {
            channel?.send(JSObject().apply {
                put("kind", "error")
                put("message", message)
            })
        }
    }
}
