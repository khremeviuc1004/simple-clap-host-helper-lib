//! Abstractions for interacting with the `state` extension.

use clap_sys::ext::posix_fd_support::{clap_plugin_posix_fd_support, clap_posix_fd_flags, CLAP_EXT_POSIX_FD_SUPPORT};
use std::ffi::{CStr};
use std::ptr::NonNull;

use super::Extension;
use crate::plugin::instance::Plugin;
use crate::util::{unsafe_clap_call};

/// Abstraction for the `posix fd support` extension covering the main thread functionality.
#[derive(Debug)]
pub struct PosixFDSupport {
    posix_fd_support: NonNull<clap_plugin_posix_fd_support>,
}

impl Extension<&Plugin> for PosixFDSupport {
    const EXTENSION_ID: &'static CStr = CLAP_EXT_POSIX_FD_SUPPORT;

    type Struct = clap_plugin_posix_fd_support;

    fn new(extension_struct: NonNull<Self::Struct>) -> Self {
        Self {
            posix_fd_support: extension_struct,
        }
    }
}

impl PosixFDSupport {
    pub fn on_fd(&self, plugin: &Plugin, fd: i32, flags: clap_posix_fd_flags) {
        unsafe_clap_call! { self.posix_fd_support.as_ptr()=>on_fd(plugin.as_ptr(), fd, flags) }
    }
}
