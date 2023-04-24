package cn.tinyhai.skmanager.ui.composelocal

import androidx.compose.runtime.compositionLocalOf
import cn.tinyhai.skmanager.ui.component.DialogHostState

val LocalDialogHost = compositionLocalOf<DialogHostState> {
    error("CompositionLocal DialogHost not present")
}