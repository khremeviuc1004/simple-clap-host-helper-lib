//! Abstractions for interacting with the `state` extension.

use clap_sys::ext::timer_support::{clap_plugin_timer_support, CLAP_EXT_TIMER_SUPPORT};
use clap_sys::id::clap_id;
use std::ffi::{CStr};
use std::ptr::NonNull;

use super::Extension;
use crate::plugin::instance::Plugin;
use crate::util::{unsafe_clap_call};

/// Abstraction for the `timer support` extension covering the main thread functionality.
#[derive(Debug)]
pub struct TimerSupport {
    timer_support: NonNull<clap_plugin_timer_support>,
}

impl Extension<&Plugin> for TimerSupport {
    const EXTENSION_ID: &'static CStr = CLAP_EXT_TIMER_SUPPORT;

    type Struct = clap_plugin_timer_support;

    fn new(extension_struct: NonNull<Self::Struct>) -> Self {
        Self {
            timer_support: extension_struct,
        }
    }
}

impl TimerSupport {
    pub fn on_timer(&self, plugin: &Plugin, timer_id: clap_id) {
        unsafe_clap_call! { self.timer_support.as_ptr()=>on_timer(plugin.as_ptr(), timer_id) }
    }
}
