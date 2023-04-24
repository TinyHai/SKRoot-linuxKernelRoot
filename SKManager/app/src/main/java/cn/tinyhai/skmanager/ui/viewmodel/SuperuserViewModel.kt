package cn.tinyhai.skmanager.ui.viewmodel

import android.app.Application
import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import cn.tinyhai.skmanager.BuildConfig
import cn.tinyhai.skmanager.ui.util.HanziToPinyin
import cn.tinyhai.skmanager.util.SPUtils
import cn.tinyhai.skmanager.util.ensureSuDeployed
import cn.tinyhai.skmanager.util.injectSu
import cn.tinyhai.skmanager.util.killApp
import cn.tinyhai.skmanager.util.startApp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import java.text.Collator
import java.util.Locale

data class AppInfo(
    val label: String,
    val packageName: String,
    val icon: PackageInfo,
)

data class SuperuserState(
    val isRefreshing: Boolean,
    val showSystemApps: Boolean,
    val appList: List<AppInfo>,
) {
    companion object {
        val Empty =
            SuperuserState(isRefreshing = false, showSystemApps = false, appList = emptyList())
    }
}

sealed interface SuperuserEffect {
    class InjectSuSuccess(val label: String) : SuperuserEffect
    object SuNotDeployed : SuperuserEffect
    class WithLoading(val onLoading: suspend () -> Unit) : SuperuserEffect
}

class SuperuserViewModel(app: Application) : AndroidViewModel(app) {
    companion object {
        private val apps = MutableStateFlow(emptyList<AppInfo>())
    }

    private val search = MutableStateFlow("")
    private val refresh = MutableStateFlow(false)
    private val showSystemApps = MutableStateFlow(false)

    val state = combine(
        search,
        refresh,
        showSystemApps,
        apps
    ) { search, isRefreshing, showSystemApps, apps ->
        SuperuserState(
            isRefreshing = isRefreshing,
            showSystemApps = showSystemApps,
            appList = apps
                .filter {
                    if (showSystemApps) {
                        true
                    } else {
                        it.icon.applicationInfo.flags.and(ApplicationInfo.FLAG_SYSTEM) == 0
                    }
                }
                .filter {
                    if (search.isEmpty()) {
                        true
                    } else {
                        it.label.contains(search)
                                || HanziToPinyin.getInstance().toPinyinString(it.label)
                            .contains(search)
                                || it.packageName.contains(search)
                    }
                }.sortedWith(compareBy(Collator.getInstance(Locale.getDefault()), AppInfo::label))
        )
    }.stateIn(viewModelScope, started = SharingStarted.WhileSubscribed(), SuperuserState.Empty)

    private val effectChannel = Channel<SuperuserEffect>()

    val effects = effectChannel.receiveAsFlow()

    private suspend fun productEffect(effect: SuperuserEffect) {
        effectChannel.send(effect)
    }

    fun getAppInfoList(force: Boolean = false) {
        if (!force && apps.value.isNotEmpty()) {
            return
        }
        refresh.value = true
        viewModelScope.launch(Dispatchers.IO) {
            val pm = getApplication<Application>().packageManager
            apps.value = pm.getInstalledPackages(0).map {
                val info = it.applicationInfo
                AppInfo(
                    label = info.loadLabel(pm).toString(), packageName = info.packageName, icon = it
                )
            }.filter { it.packageName != BuildConfig.APPLICATION_ID }
            refresh.value = false
        }
    }

    fun refresh() {
        viewModelScope.launch {
            getAppInfoList(true)
        }
    }

    fun onAppItemClick(appInfo: AppInfo) {
        viewModelScope.launch {
            productEffect(SuperuserEffect.WithLoading {
                coroutineScope {
                    if (ensureSuDeployed().not()) {
                        viewModelScope.launch {
                            productEffect(SuperuserEffect.SuNotDeployed)
                        }
                    }
                    killApp(appInfo.packageName)
                    launch {
                        delay(200)
                        startApp(packageName = appInfo.packageName)
                    }
                    val success =
                        injectSu(appInfo.packageName, suPath = SPUtils.deployPath, timeout = 10)
                    if (success) {
                        viewModelScope.launch {
                            productEffect(SuperuserEffect.InjectSuSuccess(appInfo.label))
                        }
                    }
                }
            })
        }
    }

    fun onShowSystemApps(showSystemApps: Boolean) {
        this.showSystemApps.value = showSystemApps
    }

    fun onSearchTextChange(searchText: String) {
        search.value = searchText
    }
}