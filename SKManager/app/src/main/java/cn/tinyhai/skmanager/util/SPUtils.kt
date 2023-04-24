package cn.tinyhai.skmanager.util

import android.content.SharedPreferences
import androidx.core.content.edit
import androidx.preference.PreferenceManager
import cn.tinyhai.skmanager.SKManager
import kotlin.reflect.KProperty


object SPUtils : SPHost {
    const val SP_ROOT_KEY = "sp_root_key"
    const val SP_DEPLOY_PATH = "sp_deploy_path"


    override val sp by lazy {
        PreferenceManager.getDefaultSharedPreferences(SKManager.app)
    }

    var rootKey by StringProperty(SP_ROOT_KEY, "")
    var deployPath by StringProperty(SP_DEPLOY_PATH, "")
}

private interface SPHost {
    val sp: SharedPreferences
}

private class StringProperty(private val key: String, private val default: String, private val commit: Boolean = false) {
    operator fun getValue(thisObj: SPHost, property: KProperty<*>): String {
        return thisObj.sp.getString(key, default)!!
    }

    operator fun setValue(thisObj: SPHost, property: KProperty<*>, value: String) {
        thisObj.sp.edit(commit) {
            putString(key, value)
        }
    }
}