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

### Switching by Typing
Navigate windows with [dmenu](http://tools.suckless.org/dmenu/) (requires dmenu to be installed).
Typing a few characters with the window's class, title or self-defined tag to narrow down search
scope.

![Navigation](https://raw.githubusercontent.com/kkspeed/metal-placebo/master/images/navigate.png)

## Installation
### Building and Running
Simply clone the repo and run

    cargo build

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
By default, a lot of information is emitted to stderr of the window manager. Simply redirect it to a
log file (and sorry about the messy output, I plan to fix it in the future).

### Configuration
I aim to provide interface that you can assemble your window manager on your own. But currently it's working in 
progress and the functionality is limited. See <tt>main.rs</tt> how you can modify / tweak it. Defaulting Mod key
can be done in <tt>src/config.rs</tt>, which should be fixed in the future.


### Usage
See <tt>KEYS</tt> constant in <tt>main.rs</tt> and <tt>config.rs</tt> for the list of combination keys 
and functionality.

## Warning and Future Work
This window manager is absolutely in its infancy, meaning it could crash, could leak memory and could blow up your
desktop. USE IT AT YOUR OWN RISK!

In the future, I plan to:
- Add multi-screen support.
- Clean up configuration process.
- Introduce more intuitive and cleaner logging.