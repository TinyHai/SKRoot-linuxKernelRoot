package cn.tinyhai.skmanager.ui.screen

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.InstallMobile
import androidx.compose.material.icons.filled.Key
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.lifecycle.viewmodel.compose.viewModel
import cn.tinyhai.skmanager.R
import cn.tinyhai.skmanager.ui.composelocal.LocalSnackbarHost
import cn.tinyhai.skmanager.ui.viewmodel.SettingsEffect
import cn.tinyhai.skmanager.ui.viewmodel.SettingsViewModel
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(viewModel: SettingsViewModel = viewModel()) {
    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                title = {
                    Text(text = stringResource(id = R.string.settings))
                },
                modifier = Modifier.fillMaxWidth()
            )
        }
    ) {
        SettingsItems(
            viewModel,
            modifier = Modifier
                .padding(it)
                .fillMaxWidth(),
        )
    }
}

@Composable
private fun SettingsItems(viewModel: SettingsViewModel, modifier: Modifier = Modifier) {
    val state by viewModel.state.collectAsState()
    val snackbarHost = LocalSnackbarHost.current
    val rootKeyNotSet = stringResource(id = R.string.settings_root_key_not_set)
    val suNotDeployed = stringResource(id = R.string.su_not_deployed)
    val scope = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        viewModel.effectFlow.collectLatest {
            when (it) {
                SettingsEffect.SuRemoved -> {
                    snackbarHost.showSnackbar("su has been removed")
                }
                SettingsEffect.SuDeployed -> {
                    snackbarHost.showSnackbar("su has been deployed")
                }
            }
        }
    }

    Column(modifier) {
        InputSettingsItem(
            default = state.rootKey,
            onCommit = viewModel::onRootKeyChange,
            title = stringResource(id = R.string.root_key),
            summary = stringResource(id = R.string.settings_root_key_summary),
            leading = {
                Icon(Icons.Filled.Key, contentDescription = stringResource(id = R.string.root_key))
            }
        )
        ActionSettingsItem(
            onClick = {
                if (state.rootKey.isEmpty()) {
                    scope.launch {
                        snackbarHost.showSnackbar(rootKeyNotSet, duration = SnackbarDuration.Short)
                    }
                } else {
                    viewModel.onDeploySu()
                }
            },
            title = stringResource(id = R.string.settings_deploy_su),
            summary = stringResource(id = R.string.settings_deploy_su_summary),
            leading = {
                Icon(
                    Icons.Filled.InstallMobile,
                    contentDescription = stringResource(id = R.string.settings_deploy_su)
                )
            }
        )
        ActionSettingsItem(
            onClick = {
                if (state.deployPath.isEmpty()) {
                    scope.launch {
                        snackbarHost.showSnackbar(suNotDeployed)
                    }
                } else {
                    viewModel.onRemoveSu()
                }
            },
            title = stringResource(id = R.string.settings_remove_su),
            summary = stringResource(id = R.string.settings_remove_su_summary),
            leading = {
                Icon(
                    Icons.Filled.Delete,
                    contentDescription = stringResource(id = R.string.settings_remove_su)
                )
            }
        )
    }
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun InputDialog(
    title: String,
    default: String,
    showDialog: MutableState<Boolean>,
    onCommit: (String) -> Unit,
) {
    if (showDialog.value.not()) {
        return
    }
    var inputValue by rememberSaveable(default, stateSaver = TextFieldValue.Saver) {
        mutableStateOf(TextFieldValue(default, selection = TextRange(default.length)))
    }
    val focusRequester = remember {
        FocusRequester()
    }
    val keyboardController = LocalSoftwareKeyboardController.current
    AlertDialog(
        onDismissRequest = { showDialog.value = false },
        title = {
            Text(text = title)
        },
        text = {
            DisposableEffect(Unit) {
                inputValue = inputValue.copy(selection = TextRange(inputValue.text.length))
                focusRequester.requestFocus()
                onDispose {
                    keyboardController?.hide()
                }
            }
            OutlinedTextField(
                value = inputValue,
                maxLines = 3,
                minLines = 3,
                onValueChange = {
                    inputValue = it
                },
                modifier = Modifier.focusRequester(focusRequester)
            )
        },
        confirmButton = {
            TextButton(onClick = {
                inputValue.text.let {
                    if (it != default) {
                        onCommit(it)
                    }
                }
            }) {
                Text(text = stringResource(id = android.R.string.ok))
            }
        },
        dismissButton = {
            TextButton(onClick = { showDialog.value = false }) {
                Text(text = stringResource(id = android.R.string.cancel))
            }
        }
    )
}

@Composable
private fun InputSettingsItem(
    default: String,
    onCommit: (String) -> Unit,
    title: String,
    summary: String,
    leading: @Composable () -> Unit,
    modifier: Modifier = Modifier,
) {
    val showInputDialog = rememberSaveable {
        mutableStateOf(false)
    }
    InputDialog(title = "Please input Root Key", default, showInputDialog, onCommit)

    BaseSettingsItem(
        title = title,
        summary = summary,
        leading = leading,
        modifier = modifier,
        onClick = {
            showInputDialog.value = true
        }
    )
}

@Composable
private fun ActionSettingsItem(
    onClick: () -> Unit,
    title: String,
    summary: String,
    leading: @Composable () -> Unit,
    modifier: Modifier = Modifier,
) {
    BaseSettingsItem(
        onClick = onClick,
        title = title,
        summary = summary,
        leading = leading,
        modifier = modifier,
    )
}

@Composable
private fun BaseSettingsItem(
    onClick: (() -> Unit)?,
    title: String,
    summary: String,
    modifier: Modifier = Modifier,
    leading: (@Composable () -> Unit)? = null,
) {
    val realModifier = if (onClick != null) {
        modifier.clickable { onClick() }
    } else modifier
    ListItem(
        headlineContent = {
            Text(text = title)
        },
        supportingContent = {
            Text(text = summary)
        },
        leadingContent = leading,
        modifier = realModifier
    )
}