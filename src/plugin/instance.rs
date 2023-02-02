//! Abstractions for single CLAP plugin instances for main thread interactions.

use anyhow::Result;
use clap_sys::factory::plugin_factory::clap_plugin_factory;
use clap_sys::plugin::clap_plugin;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Arc;

use super::ext::Extension;
use super::library::PluginLibrary;
use super::{assert_plugin_state_eq, assert_plugin_state_initialized};
use crate::host::{CallbackTask, Host, InstanceState};
use crate::util::unsafe_clap_call;

pub mod process;

/// A `Send+Sync` wrapper around `*const clap_plugin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PluginHandle(pub NonNull<clap_plugin>);

unsafe impl Send for PluginHandle {}
unsafe impl Sync for PluginHandle {}

/// A CLAP plugin instance. The plugin will be deinitialized when this object is dropped. All
/// functions here are callable only from the main thread. Use the
/// [`on_audio_thread()`][Self::on_audio_thread()] method to spawn an audio thread.
///
/// All functions on `Plugin` and the objects created from it will panic if the plugin is not in the
/// correct state.
#[derive(Debug)]
pub struct Plugin {
    handle: PluginHandle,
    /// Information about this plugin instance stored on the host. This keeps track of things like
    /// audio thread IDs, whether the plugin has pending callbacks, and what state it is in.
    pub state: Pin<Arc<InstanceState>>,

    /// To honor CLAP's thread safety guidelines, the thread this object was created from is
    /// designated the 'main thread', and this object cannot be shared with other threads. The
    /// [`on_audio_thread()`][Self::on_audio_thread()] method spawns an audio thread that is able to call
    /// the plugin's audio thread functions.
    _send_sync_marker: PhantomData<*const ()>,
}

/// The plugin's current lifecycle state. This is checked extensively to ensure that the plugin is
/// in the correct state, and things like double activations can't happen. `Plugin` and
/// `PluginAudioThread` will drop down to the previous state automatically when the object is
/// dropped and the stop processing or deactivate functions have not yet been calle.d
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginStatus {
    #[default]
    Uninitialized,
    Deactivated,
    Activated,
    Processing,
}

/// An unsafe `Send` wrapper around [`Plugin`], needed to create the audio thread abstraction since
/// we artifically imposed `!Send`+`!Sync` on `Plugin` using the phantomdata marker.
struct PluginSendWrapper(*const Plugin);

unsafe impl Send for PluginSendWrapper {}

impl Deref for PluginSendWrapper {
    type Target = *const Plugin;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for Plugin {
    fn drop(&mut self) {
        // Make sure the plugin is in the correct state before it gets destroyed
        match self.status() {
            PluginStatus::Uninitialized | PluginStatus::Deactivated => (),
            PluginStatus::Activated => self.deactivate(),
            status @ PluginStatus::Processing => panic!(
                "The plugin was in an invalid state '{status:?}' when the instance got dropped, \
                 this is a clap-validator bug"
            ),
        }

        // TODO: We can't handle host callbacks that happen in between these two functions, but the
        //       plugin really shouldn't be making callbacks in deactivate()
        unsafe_clap_call! { self.as_ptr()=>destroy(self.as_ptr()) };

        self.host().unregister_instance(self.state.clone());
    }
}

/// This allows methods from the CLAP plugin to be called directly independently of any
/// abstractions. All of the thread guarentees are lost when interacting with the plugin this way,
/// but that is not a problem as the function pointers are marked unsafe anyways.
impl Deref for Plugin {
    type Target = clap_plugin;

    fn deref(&self) -> &Self::Target {
        unsafe { self.handle.0.as_ref() }
    }
}

impl Plugin {
    /// Create a plugin instance and return the still uninitialized plugin. Returns an error if the
    /// plugin could not be created. The plugin instance will be registered with the host, and
    /// unregistered when this object is dropped again.
    pub fn new(
        host: Arc<Host>,
        factory: &clap_plugin_factory,
        plugin_id: &CStr,
    ) -> Result<Self> {
        // The host can use this to keep track of things like audio threads and pending callbacks.
        // The instance is remvoed again when this object is dropped.
        let state = InstanceState::new(host.clone());
        let plugin = unsafe_clap_call! {
            factory=>create_plugin(factory, state.clap_host_ptr(), plugin_id.as_ptr())
        };
        if plugin.is_null() {
            anyhow::bail!(
                "'clap_plugin_factory::create_plugin({plugin_id:?})' returned a null pointer"
            );
        }

        // We can only register the plugin instance with the host now because we did not have a
        // plugin pointer before this.
        let handle = PluginHandle(NonNull::new(plugin as *mut clap_plugin).unwrap());
        state.plugin.store(Some(handle));
        host.register_instance(state.clone());

        Ok(Plugin {
            handle,
            state,

            _send_sync_marker: PhantomData,
        })
    }

    /// Get the raw pointer to the `clap_plugin` instance.
    pub fn as_ptr(&self) -> *const clap_plugin {
        self.handle.0.as_ptr()
    }

    /// Get the host for this plugin instance.
    pub fn host(&self) -> &Host {
        // `Plugin` can only be used from the main thread
        self.state
            .host()
            .expect("Tried to get the host instance from a thread that isn't the main thread")
    }

    /// The plugin's current initialization status.
    pub fn status(&self) -> PluginStatus {
        self.state.status.load()
    }

    /// Get the _main thread_ extension abstraction for the extension `T`, if the plugin supports
    /// this extension. Returns `None` if it does not. The plugin needs to be initialized using
    /// [`init()`][Self::init()] before this may be called.
    pub fn get_extension<'a, T: Extension<&'a Self>>(&'a self) -> Option<T> {
        assert_plugin_state_initialized!(self);

        let extension_ptr = unsafe_clap_call! {
            self.as_ptr()=>get_extension(self.as_ptr(), T::EXTENSION_ID.as_ptr())
        };

        if extension_ptr.is_null() {
            None
        } else {
            Some(T::new(
                NonNull::new(extension_ptr as *mut T::Struct).unwrap(),
            ))
        }
    }

    /// Initialize the plugin. This needs to be called before doing anything else.
    pub fn init(&self) -> Result<()> {
        assert_plugin_state_eq!(self, PluginStatus::Uninitialized);

        if unsafe_clap_call! { self.as_ptr()=>init(self.as_ptr()) } {
            self.state.status.store(PluginStatus::Deactivated);
            Ok(())
        } else {
            anyhow::bail!("'clap_plugin::init()' returned false")
        }
    }

    /// Activate the plugin. Returns an error if the plugin returned `false`. See
    /// [plugin.h](https://github.com/free-audio/clap/blob/main/include/clap/plugin.h) for the
    /// preconditions.
    pub fn activate(
        &self,
        sample_rate: f64,
        min_buffer_size: usize,
        max_buffer_size: usize,
    ) -> Result<()> {
        assert_plugin_state_eq!(self, PluginStatus::Deactivated);

        // Apparently 0 is invalid here
        assert!(min_buffer_size >= 1);

        if unsafe_clap_call! {
            self.as_ptr()=>activate(
                self.as_ptr(),
                sample_rate,
                min_buffer_size as u32,
                max_buffer_size as u32,
            )
        } {
            self.state.status.store(PluginStatus::Activated);
            Ok(())
        } else {
            anyhow::bail!("'clap_plugin::activate()' returned false")
        }
    }

    /// Deactivate the plugin. See
    /// [plugin.h](https://github.com/free-audio/clap/blob/main/include/clap/plugin.h) for the
    /// preconditions.
    pub fn deactivate(&self) {
        assert_plugin_state_eq!(self, PluginStatus::Activated);

        unsafe_clap_call! { self.as_ptr()=>deactivate(self.as_ptr()) };

        self.state.status.store(PluginStatus::Deactivated);
    }
}
