package cn.tinyhai.skmanager.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material.icons.outlined.Security
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import cn.tinyhai.skmanager.R
import cn.tinyhai.skmanager.ui.component.rememberDialogHostState
import cn.tinyhai.skmanager.ui.composelocal.LocalDialogHost
import cn.tinyhai.skmanager.ui.composelocal.LocalSnackbarHost
import cn.tinyhai.skmanager.ui.theme.SKManagerTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            SKManagerTheme {
                val navController = rememberNavController();
                val snackbarHostState = remember { SnackbarHostState() }
                Scaffold(
                    bottomBar = {
                        BottomBar(navController)
                    },
                    snackbarHost = {
                        SnackbarHost(hostState = snackbarHostState)
                    }
                ) {
                    CompositionLocalProvider(
                        LocalDialogHost provides rememberDialogHostState(),
                        LocalSnackbarHost provides snackbarHostState,
                    ) {
                        SKNavHost(
                            navController,
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(it),
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun BottomBar(navController: NavController) {
    val topDestination =
        navController.currentBackStackEntryAsState().value?.destination?.route ?: Screen.Superuser.route

    NavigationBar(tonalElevation = 8.dp) {
        NavigationBarItem(
            selected = topDestination == Screen.Superuser.route,
            onClick = {
                navController.navigate(Screen.Superuser.route) {
                    popUpTo(navController.graph.findStartDestination().id) {
                        saveState = true
                    }
                    launchSingleTop = true
                    restoreState = true
                }
            },
            icon = {
                if (topDestination == Screen.Superuser.route) {
                    Icon(
                        Icons.Filled.Security,
                        contentDescription = stringResource(id = R.string.superuser)
                    )
                } else {
                    Icon(
                        Icons.Outlined.Security,
                        contentDescription = stringResource(id = R.string.superuser)
                    )
                }
            },
            label = {
                Text(text = stringResource(id = R.string.superuser))
            },
            alwaysShowLabel = false,
        )

        NavigationBarItem(
            selected = topDestination == Screen.Settings.route,
            onClick = {
                navController.navigate(Screen.Settings.route) {
                    popUpTo(navController.graph.findStartDestination().id) {
                        saveState = true
                    }
                    launchSingleTop = true
                    restoreState = true
                }
            },
            icon = {
                if (topDestination == Screen.Settings.route) {
                    Icon(
                        Icons.Filled.Settings,
                        contentDescription = stringResource(id = R.string.settings)
                    )
                } else {
                    Icon(
                        Icons.Outlined.Settings,
                        contentDescription = stringResource(id = R.string.settings)
                    )
                }
            },
            label = {
                Text(text = stringResource(id = R.string.settings))
            },
            alwaysShowLabel = false,
        )
    }
}