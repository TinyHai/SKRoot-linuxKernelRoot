package cn.tinyhai.skmanager

import android.app.Application
import android.widget.Toast
import coil.Coil
import coil.ImageLoader
import me.zhanghai.android.appiconloader.coil.AppIconFetcher
import me.zhanghai.android.appiconloader.coil.AppIconKeyer

class SKManager : Application() {
    override fun onCreate() {
        super.onCreate()
        app = this

        val context = this
        val iconSize = resources.getDimensionPixelSize(android.R.dimen.app_icon_size)
        Coil.setImageLoader(
            ImageLoader.Builder(context)
                .components {
                    add(AppIconKeyer())
                    add(AppIconFetcher.Factory(iconSize, false, context))
                }
                .build()
        )
    }

    companion object {
        lateinit var app: Application

        fun toast(msg: String) {
            Toast.makeText(app, msg, Toast.LENGTH_SHORT).show()
        }
    }
}