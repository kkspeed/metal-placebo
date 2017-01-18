By default, display managers run <tt>.xprofile</tt> to initialize
start up programs prior to starting the window manager. Thus, 
you can place all the auto start programms here, especially those
you don't want the window manager to take care (otherwise, you should
place them in the <tt>main.rs</tt> instead).

This folder contains my current configurations.

I use <tt>rxvt-unicode</tt> as my terminal. The <tt>.Xresources</tt>
contains default configuration for <tt>rxvt-unicode</tt>. Other auto
start programs include:

| Program    | Description                              |
|------------|------------------------------------------|
| trayer     | Systray                                  |
| volti      | Volume control on systray                |
| xautolock  | Auto locks the system                    |
| i3lock     | Used by xautolock to lock screen         |
| nm-applet  | NetworkManager frontend                  |
| parcellite | Clipboard manager                        |

I use xmobar as the system bar (and it's the only supported so far for
the window manager). The configuration is included in <tt>.xmobarrc</tt>.

The <tt>rswm.desktop</tt> file is the entry for your display manager.
After putting <tt>rswm</tt> executable on your PATH, place <tt>rswm.desktop</tt>
in <tt>/usr/share/xsessions</tt>

To set your wallpaper, you can use <tt>feh</tt>. Place the following line
in <tt>.xprofile</tt>.

    feh --bg-fill <path to your image>

