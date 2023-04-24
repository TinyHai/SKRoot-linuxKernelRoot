package cn.tinyhai.skmanager.util

import android.util.Log
import cn.tinyhai.skmanager.SKManager
import com.topjohnwu.superuser.ShellUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.internal.closeQuietly
import okio.IOException
import java.io.File

private const val TAG = "SKCli"

internal val skCliPath by lazy {
    SKManager.app.applicationInfo.nativeLibraryDir + File.separator + "libsk_cli.so"
}

private val tmpPath by lazy {
    "/data/local/tmp"
}

private suspend fun extractSuToCache(): File {
    val context = SKManager.app
    val cacheSu = File(context.cacheDir, "su")
    if (cacheSu.exists()) {
        return cacheSu
    }
    val suStream = context.assets.open("su")
    val bufferInput = suStream.buffered()
    val bufferOutput = cacheSu.outputStream().buffered()
    withContext(Dispatchers.IO) {
        try {
            bufferInput.copyTo(bufferOutput)
        } catch (e: IOException) {
            e.printStackTrace()
            throw e
        } finally {
            bufferInput.closeQuietly()
            bufferOutput.closeQuietly()
        }
    }

    return cacheSu
}

suspend fun deploySu(): String {
    return withContext(Dispatchers.IO) {
        val cacheSu = extractSuToCache()
        val shell = getNormalShell()
        val shellCmd = ArrayList<String>().apply {
            add(skCliPath)
            add("deploy")
            add(SPUtils.rootKey)
            add(cacheSu.absolutePath)
            add(tmpPath)
        }
        val stdout = ArrayList<String>()
        val stderr = ArrayList<String>()
        val result = shell.newJob().add(shellCmd.joinToString(" ")).to(stdout, stderr).exec()
        if (result.isSuccess) {
            result.out.filter { it.isNotBlank() }[0] ?: ""
        } else {
            ""
        }
    }
}

suspend fun removeSu(deployPath: String): Boolean =
    withContext(Dispatchers.IO) {
        runWithRootShell {
            if (deployPath.startsWith(tmpPath)) {
                ShellUtils.fastCmdResult(this, "rm -rf $deployPath")
            } else {
                Log.w(TAG, "try delete path $deployPath denied")
                false
            }
        }
    }

suspend fun injectSu(cmd: String, suPath: String, timeout: Int): Boolean {
    return withContext(Dispatchers.IO) {
        val shell = newRootShell()
        val shellCmd = ArrayList<String>().apply {
            add(skCliPath)
            add("inject")
            if (shell.isRoot.not()) {
                add("-t")
                add(SPUtils.rootKey)
            }
            add("-c")
            add(cmd)
            add("-s")
            add(suPath)
            add("-t")
            add(timeout.toString())
        }
        val stdout = ArrayList<String>()
        val stderr = ArrayList<String>()
        val result = shell.newJob().add(shellCmd.joinToString(" ")).to(stdout, stderr).exec()
        if (result.isSuccess) {
            true
        } else {
            Log.d(TAG, result.err.toString())
            false
        }
    }
}
