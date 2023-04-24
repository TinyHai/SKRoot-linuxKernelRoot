package cn.tinyhai.skmanager.util

import com.topjohnwu.superuser.Shell
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

suspend fun ensureSuDeployed() = withContext(Dispatchers.IO) {
    val deployPath = SPUtils.deployPath
    if (deployPath.isEmpty()) {
        false
    } else {
        try {
            val shell = Shell.Builder.create().build("$deployPath/su")
            shell.isRoot
        } catch (_: Exception) {
            false
        }
    }
}