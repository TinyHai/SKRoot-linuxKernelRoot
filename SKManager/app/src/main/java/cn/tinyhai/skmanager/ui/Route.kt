package cn.tinyhai.skmanager.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import cn.tinyhai.skmanager.ui.screen.SettingsScreen
import cn.tinyhai.skmanager.ui.screen.SuperuserScreen

sealed interface BottomNavigation {
    val route: String
}

sealed class Screen(val route: String) {
    object Superuser : Screen("superuser"), BottomNavigation

    object Settings : Screen("settings"), BottomNavigation
}


@Composable
fun SKNavHost(navController: NavHostController, modifier: Modifier = Modifier) {
    NavHost(
        navController = navController,
        startDestination = Screen.Superuser.route,
        modifier = modifier
    ) {
        composable(Screen.Superuser.route) {
            SuperuserScreen()
        }
        composable(Screen.Settings.route) {
            SettingsScreen()
        }
    }
}