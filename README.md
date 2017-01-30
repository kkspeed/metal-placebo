# Metal Placebo
A naive tiling window manager largely inspired by DWM. It's called metal because it kind of fits
into the naming idiology of Rust programs. I't called placebo, because, it just gives you a sense
of reassurance that it's a working x11 window manager.

![Tiling](https://raw.githubusercontent.com/kkspeed/metal-placebo/master/images/tiling.png)

## Features
### Overview Mode
Overview mode gives a view of all windows, which provides a more intuive
interface to find windows of interest.

![Overview](https://raw.githubusercontent.com/kkspeed/metal-placebo/master/images/overview.png)

or even:

![Overview](https://raw.githubusercontent.com/kkspeed/metal-placebo/master/images/overview_2.png)

### Switching by Typing
Navigate windows with [dmenu](http://tools.suckless.org/dmenu/) (requires dmenu to be installed).
Typing a few characters with the window's class, title or self-defined tag to narrow down search
scope.

![Navigation](https://raw.githubusercontent.com/kkspeed/metal-placebo/master/images/navigate.png)

## Installation
### Building and Running
Simply clone the repo and run

    cargo build

It's recommended that you build with **Rust nightly (at least 1.14)** since there is some discrepancy
in FFI callback syntax.

Put <tt>target/debug/rswm</tt> in your path.
- For <tt>.xinitrc</tt>, put <tt> exec rswm</tt> in your xinitrc.
- If you are using a display manager (DM), put the following code in <tt>/usr/share/xsessions/rswm.desktop</tt>:

~~~
    [Desktop Entry]
    Encoding=UTF-8
    Name=RSWM
    Type=Application
    Exec=rswm
    Comment=A naive tiling window manager
~~~

### Trouble Shotting
By default, the window manager emits log to <tt>~/rswm_error.log</tt>. You can modify this in <tt>init_logging</tt>
in <tt>main.rs</tt>.

### Configuration
I aim to provide the interface that you can assemble your window manager on your own. But currently it's working in
progress and the functionality is limited. <tt>main.rs</tt> is what I'm using on my own machine. It's highly personally
flavoured but could be a reference point for your configuration.See <tt>main.rs</tt> how you can modify / tweak it.


### Usage
#### Keys defined in <tt>src/config.rs</tt>

| Keys                 | Functionality                                                                                |
|----------------------|----------------------------------------------------------------------------------------------|
| Mod4 + Q             | Quit                                                                                         |
| Mod4 + J             | Focus next window                                                                            |
| Mod4 + K             | Focus previous window                                                                        |
| Mod4 + F4            | Kill window                                                                                  |
| Mod4 + M             | Maximize window (only work for non-floating window)                                          |
| Mod4 + Left arrow    | Move window left (only work for floating window)                                             |
| Mod4 + Right arrow   | Move window right (only work for floating window)                                            |
| Mod4 + Up arrow      | Move window up (only work for floating window)                                               |
| Mod4 + Down arrow    | Move window down (only work for floating window)                                             |
| Mod4 + Shift + Up    | Reduce window height (only work for floating window)                                         |
| Mod4 + Shift + Down  | Increase window height (only work for floating window)                                       |
| Mod4 + Shift + Left  | Reduce window width (only work for floating window)                                          |
| Mod4 + Shift + Right | Increase window width (only work for floating window)                                        |
| Mod4 + F2            | Go to overview                                                                               |
| Mod4 + Return        | Bump current window to the 1st in client list. Switch to current window if in overview mode. |
| Mod4 + 1 - 9         | Go to tag 1 - 9                                                                              |
| Mod4 + Mouse1        | Move window (only work for floating window)                                                  |
| Mod4 + Mouse3        | Resize window (only work for floating window)                                                |
| Mod4 + E             | Toggle floating / tiled state of focused window.                                             |

#### Keys defined in <tt>main.rs</tt>
To enjoy full functionality, you should install the corresponding packages.

| Key                | Functionality                                                    |
|--------------------|------------------------------------------------------------------|
| Mod4 + R           | spawn <tt>dmenu_run</tt>                                         |
| Mod4 + T           | spawn <tt>urxvt</tt>                                             |
| Mod4 + Print       | spawn <tt>scrot</tt> for screenshot                              |
| Mod4 + Alt + Print | spawn <tt>scrot</tt> for screnshot by allowing selecting regions |
| Mod4 + F           | spawn <tt>pcmanfm</tt>                                           |
| Mod4 + L           | spawn <tt>i3lock</tt>                                            |
| Mod4 + Shift + T   | add tag annotation to current workspace                          |
| Mod4 + Shift + W   | add tag annotation to current window                             |
| Mod4 + W           | look for window by typing                                        |
| Mod4 + Y           | focus the 1st client in current workspace                        |
| Mod4 + U           | focus the 2nd client in current workspace                        |
| Mod4 + I           | focus the 3rd client in current workspace                        |
| Mod4 + O           | focus the 4th client in current workspace                        |
| Mod4 + P           | focus the last client in current workspace                       |
| Mod4 + Tab         | alters between 2 recent workspaces                               |

For more information, see <tt>KEYS</tt> constant in <tt>main.rs</tt> and <tt>config.rs</tt> for the list of combination keys
and functionality.

#### Initial Spawning Programs
The window manager spawns a bunch of programs, defined in <tt>main.rs</tt>. It's highly likely you want to delete / add
your own. The defined ones in <tt>main.rs</tt> are:

| Command              | Description                                                                                |
|----------------------|----------------------------------------------------------------------------------------------|
| xcompmgr             | allow transparency  |
| fcitx                | input method |
| tilda                | dropdown terminal |
| polkit agent kde     | polkit tool |



## Warning and Future Work
This window manager is absolutely in its infancy, meaning it could crash, could leak memory and could blow up your
desktop. USE IT AT YOUR OWN RISK!

In the future, I plan to:
- Add multi-screen support.
- Clean up configuration process.
- Introduce more intuitive and cleaner logging.
