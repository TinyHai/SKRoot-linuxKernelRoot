package cn.tinyhai.skmanager.ui.screen

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.ExperimentalMaterialApi
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.pullrefresh.PullRefreshIndicator
import androidx.compose.material.pullrefresh.pullRefresh
import androidx.compose.material.pullrefresh.rememberPullRefreshState
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import cn.tinyhai.skmanager.R
import cn.tinyhai.skmanager.SKManager
import cn.tinyhai.skmanager.ui.component.LoadingDialog
import cn.tinyhai.skmanager.ui.composelocal.LocalDialogHost
import cn.tinyhai.skmanager.ui.composelocal.LocalSnackbarHost
import cn.tinyhai.skmanager.ui.viewmodel.AppInfo
import cn.tinyhai.skmanager.ui.viewmodel.SuperuserEffect
import cn.tinyhai.skmanager.ui.viewmodel.SuperuserViewModel
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.flow.collectLatest

@Composable
fun SuperuserScreen(viewModel: SuperuserViewModel = viewModel()) {
    val state by viewModel.state.collectAsState()
    val grantRootSuccess = stringResource(id = R.string.superuser_grant_root_success)
    val dialogHost = LocalDialogHost.current
    val snackbarHost = LocalSnackbarHost.current
    val suNotDeployed = stringResource(id = R.string.su_not_deployed)
    LaunchedEffect(Unit) {
        viewModel.effects.collectLatest {
            when (it) {
                is SuperuserEffect.InjectSuSuccess -> {
                    SKManager.toast(grantRootSuccess.format(it.label))
                }

                is SuperuserEffect.WithLoading -> {
                    dialogHost.withLoading {
                        it.onLoading()
                    }
                }

                SuperuserEffect.SuNotDeployed -> snackbarHost.showSnackbar(
                    suNotDeployed,
                    duration = SnackbarDuration.Short
                )
            }
        }
    }

    Scaffold(
        topBar = {
            TopBar(state.showSystemApps, viewModel::onShowSystemApps, viewModel::onSearchTextChange)
        },
        modifier = Modifier.fillMaxSize()
    ) {
        LoadingDialog()

        LaunchedEffect(Unit) {
            viewModel.getAppInfoList()
        }
        AppList(
            isRefreshing = state.isRefreshing,
            appList = state.appList,
            onRefreshing = viewModel::refresh,
            onItemClick = viewModel::onAppItemClick,
            modifier = Modifier
                .padding(it)
                .fillMaxSize()
        )
    }
}

@OptIn(ExperimentalComposeUiApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun SearchAppBar(
    title: @Composable () -> Unit,
    onSearchTextChange: (String) -> Unit,
    onBackClick: (() -> Unit)? = null,
    onConfirm: (() -> Unit)? = null,
    dropdownContent: @Composable (() -> Unit)? = null,
) {
    val keyboardController = LocalSoftwareKeyboardController.current
    val focusRequester = remember { FocusRequester() }
    var onSearch by remember { mutableStateOf(false) }

    LaunchedEffect(onSearch) {
        if (onSearch) {
            focusRequester.requestFocus()
        }
    }
    DisposableEffect(Unit) {
        onDispose {
            keyboardController?.hide()
        }
    }

    val searchText = rememberSaveable {
        mutableStateOf("")
    }

    TopAppBar(
        title = {
            Box {
                AnimatedVisibility(
                    modifier = Modifier.align(Alignment.CenterStart),
                    visible = !onSearch,
                    enter = fadeIn(),
                    exit = fadeOut(),
                    content = { title() }
                )

                AnimatedVisibility(
                    visible = onSearch,
                    enter = fadeIn(),
                    exit = fadeOut()
                ) {
                    OutlinedTextField(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(
                                top = 2.dp,
                                bottom = 2.dp,
                                end = if (onBackClick != null) 0.dp else 14.dp
                            )
                            .focusRequester(focusRequester)
                            .onFocusChanged { focusState ->
                                if (focusState.isFocused) onSearch = true
                            },
                        value = searchText.value,
                        onValueChange = {
                            searchText.value = it
                            onSearchTextChange(it)
                        },
                        trailingIcon = {
                            IconButton(
                                onClick = {
                                    onSearch = false
                                    keyboardController?.hide()
                                    searchText.value = ""
                                    onSearchTextChange("")
                                },
                                content = { Icon(Icons.Filled.Close, null) }
                            )
                        },
                        maxLines = 1,
                        singleLine = true,
                        keyboardOptions = KeyboardOptions.Default.copy(imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(
                            onDone = {
                                keyboardController?.hide()
                                onConfirm?.invoke()
                            },
                        )
                    )
                }
            }
        },
        navigationIcon = {
            if (onBackClick != null) {
                IconButton(
                    onClick = onBackClick,
                    content = { Icon(Icons.Outlined.ArrowBack, null) }
                )
            }
        },
        actions = {
            AnimatedVisibility(
                visible = !onSearch
            ) {
                IconButton(
                    onClick = { onSearch = true },
                    content = { Icon(Icons.Filled.Search, null) }
                )
            }

            if (dropdownContent != null) {
                dropdownContent()
            }
        }
    )
}

@Composable
private fun TopBar(
    showSystemApps: Boolean,
    onShowSystemApps: (Boolean) -> Unit,
    onSearchTextChange: (String) -> Unit
) {
    SearchAppBar(
        title = { Text(stringResource(R.string.superuser)) },
        onSearchTextChange = onSearchTextChange,
        dropdownContent = {
            var showDropdown by remember { mutableStateOf(false) }

            IconButton(
                onClick = { showDropdown = true },
            ) {
                Icon(
                    imageVector = Icons.Filled.MoreVert,
                    contentDescription = stringResource(id = R.string.settings)
                )

                DropdownMenu(
                    expanded = showDropdown,
                    onDismissRequest = {
                        showDropdown = false
                    },
                ) {
                    DropdownMenuItem(
                        text = {
                            Text(
                                if (showSystemApps) {
                                    stringResource(R.string.hide_system_apps)
                                } else {
                                    stringResource(R.string.show_system_apps)
                                }
                            )
                        },
                        onClick = {
                            onShowSystemApps(showSystemApps.not())
                            showDropdown = false
                        },
                    )
                }
            }
        },
    )
}

@OptIn(ExperimentalMaterialApi::class)
@Composable
private fun AppList(
    isRefreshing: Boolean,
    appList: List<AppInfo>,
    onRefreshing: () -> Unit,
    onItemClick: (AppInfo) -> Unit,
    modifier: Modifier = Modifier
) {
    val pullRefreshState = rememberPullRefreshState(isRefreshing, onRefresh = onRefreshing)
    Box(modifier.pullRefresh(pullRefreshState)) {
        LazyColumn(modifier = Modifier.fillMaxSize()) {
            items(appList, key = AppInfo::packageName) {
                AppItem(appInfo = it, modifier = Modifier.clickable { onItemClick(it) })
            }
        }
        PullRefreshIndicator(
            refreshing = isRefreshing,
            state = pullRefreshState,
            modifier = Modifier.align(
                Alignment.TopCenter
            )
        )
    }
}

@Composable
private fun AppItem(appInfo: AppInfo, modifier: Modifier = Modifier) {
    ListItem(
        headlineContent = {
            Text(text = appInfo.label)
        },
        supportingContent = {
            Text(text = appInfo.packageName)
        },
        leadingContent = {
            AsyncImage(
                model = ImageRequest.Builder(LocalContext.current)
                    .data(appInfo.icon)
                    .crossfade(true)
                    .build(),
                contentDescription = appInfo.label,
                modifier = Modifier
                    .padding(4.dp)
                    .size(48.dp)
            )
        },
        modifier = modifier
    )
}