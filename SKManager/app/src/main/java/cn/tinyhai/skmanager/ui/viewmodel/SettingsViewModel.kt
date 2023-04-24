package cn.tinyhai.skmanager.ui.viewmodel

import android.content.SharedPreferences.OnSharedPreferenceChangeListener
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import cn.tinyhai.skmanager.util.SPUtils
import cn.tinyhai.skmanager.util.deploySu
import cn.tinyhai.skmanager.util.removeSu
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class SettingsState(
    val rootKey: String,
    val deployPath: String,
) {
    companion object {
        val Empty = SettingsState("", "")
    }
}

sealed interface SettingsEffect {
    object SuRemoved : SettingsEffect
    object SuDeployed : SettingsEffect
}

class SettingsViewModel : ViewModel() {

    private val rootKey = MutableStateFlow("")
    private val deployPath = MutableStateFlow("")

    val state = combine(rootKey, deployPath) { rootKey, deployPath ->
        SettingsState(
            rootKey = rootKey,
            deployPath = deployPath,
        )
    }.stateIn(viewModelScope, started = SharingStarted.WhileSubscribed(), SettingsState.Empty)

    private val effectChannel = Channel<SettingsEffect>()

    val effectFlow = effectChannel.receiveAsFlow()

    private suspend fun productEffect(effect: SettingsEffect) {
        effectChannel.send(effect)
    }

    private val spListener = OnSharedPreferenceChangeListener { _, key ->
        when (key) {
            SPUtils.SP_ROOT_KEY -> {
                rootKey.value = SPUtils.rootKey
            }
            SPUtils.SP_DEPLOY_PATH -> {
                deployPath.value = SPUtils.deployPath
            }
        }
    }

    init {
        SPUtils.sp.registerOnSharedPreferenceChangeListener(spListener)
        viewModelScope.launch(Dispatchers.IO) {
            rootKey.value = SPUtils.rootKey
            deployPath.value = SPUtils.deployPath
        }
    }

    override fun onCleared() {
        super.onCleared()
        SPUtils.sp.unregisterOnSharedPreferenceChangeListener(spListener)
    }

    fun onRootKeyChange(rootKey: String) {
        viewModelScope.launch {
            SPUtils.rootKey = rootKey
        }
    }

    fun onDeploySu() {
        viewModelScope.launch {
            SPUtils.deployPath = deploySu()
            productEffect(SettingsEffect.SuDeployed)
        }
    }

    fun onRemoveSu() {
        viewModelScope.launch {
            removeSu(deployPath.value)
            SPUtils.deployPath = ""
            productEffect(SettingsEffect.SuRemoved)
        }
    }
}