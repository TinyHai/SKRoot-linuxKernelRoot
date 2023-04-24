package cn.tinyhai.skmanager.util

import com.topjohnwu.superuser.Shell
import com.topjohnwu.superuser.ShellUtils


private val normaShell = createNormalShell()

private var rootShell = normaShell


private fun createNormalShell(): Shell {
    val builder = Shell.Builder.create()
    return builder.build("sh")
}

private fun createRootShell(rootKey: String): Shell {
    if (rootKey.isEmpty()) {
        return normaShell
    }
    val builder = Shell.Builder.create()
    return try {
        builder.build(skCliPath, "su", rootKey)
    } catch (_: Throwable) {
        normaShell
    }
}

fun getRootShell(): Shell {
    if (rootShell.isRoot) {
        return rootShell
    }
    val rootKey = SPUtils.rootKey
    rootShell = createRootShell(rootKey)
    return rootShell
}

fun newRootShell(): Shell {
    return createRootShell(SPUtils.rootKey)
}

fun getNormalShell(): Shell {
    return normaShell
}

fun <T> runWithRootShell(block: Shell.() -> T) = getRootShell().block()