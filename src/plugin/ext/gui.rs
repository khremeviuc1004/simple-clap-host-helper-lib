//! Abstractions for interacting with the `state` extension.

use clap_sys::ext::gui::{clap_plugin_gui, CLAP_EXT_GUI, clap_window, clap_gui_resize_hints};
use std::ffi::{CStr, c_char};
use std::ptr::NonNull;

use super::Extension;
use crate::plugin::instance::Plugin;
use crate::util::{unsafe_clap_call};

/// Abstraction for the `GUI support` extension covering the main thread functionality.
#[derive(Debug)]
pub struct Gui {
    gui: NonNull<clap_plugin_gui>,
}

impl Extension<&Plugin> for Gui {
    const EXTENSION_ID: &'static CStr = CLAP_EXT_GUI;

    type Struct = clap_plugin_gui;

    fn new(extension_struct: NonNull<Self::Struct>) -> Self {
        Self {
            gui: extension_struct,
        }
    }
}

impl Gui {
    pub fn is_api_supported(&self, plugin: &Plugin, api: &CStr, is_floating: bool,) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>is_api_supported(plugin.as_ptr(), api.as_ptr(), is_floating) }
    }
    pub fn get_preferred_api(&self, plugin: &Plugin, is_floating: &mut bool,) -> String {
        // unsafe_clap_call! { self.gui.as_ptr()=>get_preferred_api(plugin.as_ptr(), api, is_floating) }
        String::from("")
    }
    pub fn create(&self, plugin: &Plugin, api: &CStr, is_floating: bool,) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>create(plugin.as_ptr(), api.as_ptr(), is_floating) }
    }
    pub fn destroy(&self, plugin: &Plugin) {
        unsafe_clap_call! { self.gui.as_ptr()=>destroy(plugin.as_ptr()) }
    }
    pub fn set_scale(&self, plugin: &Plugin, scale: f64) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>set_scale(plugin.as_ptr(), scale) }
    }
    pub fn get_size(&self, plugin: &Plugin, width: &mut u32, height: &mut u32) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>get_size(plugin.as_ptr(), width, height) }
    }
    pub fn can_resize(&self, plugin: &Plugin) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>can_resize(plugin.as_ptr()) }
    }
    pub fn get_resize_hints(&self, plugin: &Plugin, hints: &mut clap_gui_resize_hints) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>get_resize_hints(plugin.as_ptr(), hints) }
    }
    pub fn adjust_size(&self, plugin: &Plugin, width: &mut u32, height: &mut u32) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>adjust_size(plugin.as_ptr(), width, height) }
    }
    pub fn set_size(&self, plugin: &Plugin, width: u32, height: u32) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>set_size(plugin.as_ptr(), width, height) }
    }
    pub fn set_parent(&self, plugin: &Plugin, window: &clap_window) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>set_parent(plugin.as_ptr(), window) }
    }
    pub fn set_transient(&self, plugin: &Plugin, window: &clap_window) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>set_transient(plugin.as_ptr(), window) }
    }
    pub fn suggest_title(&self, plugin: &Plugin, title: *const c_char) {
        unsafe_clap_call! { self.gui.as_ptr()=>suggest_title(plugin.as_ptr(), title) }
    }
    pub fn show(&self, plugin: &Plugin) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>show(plugin.as_ptr()) }
    }
    pub fn hide(&self, plugin: &Plugin) -> bool {
        unsafe_clap_call! { self.gui.as_ptr()=>hide(plugin.as_ptr()) }
    }
}
