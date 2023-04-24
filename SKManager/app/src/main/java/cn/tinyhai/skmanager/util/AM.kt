package cn.tinyhai.skmanager.util

import android.content.Intent
import android.content.pm.PackageManager
import android.util.Log
import cn.tinyhai.skmanager.SKManager
import com.topjohnwu.superuser.ShellUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.jetbrains.annotations.TestOnly

private const val TAG = "AM"

private val pm: PackageManager
    get() = SKManager.app.packageManager

private val launchableIntent by lazy {
    Intent(Intent.ACTION_MAIN, null).apply {
        addCategory(Intent.CATEGORY_LAUNCHER)
    }
}

private fun findLauncherActivityFor(packageName: String): String? {
    val launchables = pm.queryIntentActivities(launchableIntent, 0)
    return launchables.find { it.activityInfo.packageName == packageName }?.activityInfo?.name
}

suspend fun killApp(packageName: String): Boolean = withContext(Dispatchers.IO) {
    runWithRootShell {
        val shellCmd = arrayOf(
            "am",
            "force-stop",
            packageName
        )
        ShellUtils.fastCmdResult(this, shellCmd.joinToString(" "))
    }
}

suspend fun startApp(packageName: String): Boolean {
    val intent = pm.getLaunchIntentForPackage(packageName)
    val activityName = intent?.component?.className ?: findLauncherActivityFor(packageName)
    return if (activityName != null) {
        launchAppByAM(packageName, activityName) || launchAppByNormal(packageName, activityName)
    } else {
        false
    }
}

private suspend fun launchAppByAM(packageName: String, activityName: String): Boolean =
    withContext(Dispatchers.IO) {
        runWithRootShell {
            val shellCmd = ArrayList<String>().apply {
                add("am")
                add("start")
                add("-n")
                add("$packageName/$activityName")
            }
            ShellUtils.fastCmdResult(this, shellCmd.joinToString(" "))
        }
    }

private suspend fun launchAppByNormal(packageName: String, activityName: String): Boolean =
    withContext(Dispatchers.Main.immediate) {
        val intent = Intent(Intent.ACTION_MAIN).apply {
            addCategory(Intent.CATEGORY_LAUNCHER)
            setClassName(packageName, activityName)
            flags = Intent.FLAG_ACTIVITY_NEW_TASK
        }
        try {
            SKManager.app.startActivity(intent)
            true
        } catch (e: Exception) {
            e.printStackTrace()
            false
        }
    }

@TestOnly
suspend fun startAppTest(packageName: String) {
    val intent = pm.getLaunchIntentForPackage(packageName)
    Log.d(TAG, intent.toString())
    val activityName = intent?.component?.className ?: findLauncherActivityFor(packageName)
    if (activityName != null) {
        launchAppByAM(packageName, activityName)
    }
}