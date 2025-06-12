//! flash.media.Camera native implementations

use crate::avm2::activation::Activation;
use crate::avm2::error::Error;
use crate::avm2::object::{ArrayObject, Object, TObject, ObjectPtrTestExt, ClassObject};
use crate::avm2::value::Value;
use gc_arena::{MutationContext, GcCell};
use crate::avm2::class::Class;
use crate::avm2::method::{Method, NativeMethodImpl};
use crate::avm2::names::{Namespace, QName};
use crate::avm2::string::AvmString;

#[cfg(target_os = "linux")]
use v4l::{Device, capability::Flags as CapFlags};
#[cfg(target_os = "linux")]
use tracing::{warn, info};


/// Placeholder for the Camera constructor
pub fn camera_constructor<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(Value::Undefined)
}

/// Implements `flash.media.Camera.names` static getter
pub fn get_camera_names<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let names_vec: Vec<AvmString<'gc>> = {
        #[cfg(target_os = "linux")]
        {
            let mut linux_names: Vec<AvmString<'gc>> = Vec::new();
            for i in 0..10 {
                match Device::new(i) {
                    Ok(device) => {
                        match device.query_caps() {
                            Ok(caps) => {
                                if caps.capabilities.contains(CapFlags::VIDEO_CAPTURE) {
                                    let card_name = String::from_utf8_lossy(&caps.card)
                                        .trim_end_matches('\0')
                                        .to_string();
                                    linux_names.push(AvmString::new_utf8(activation.context.gc_context, card_name));
                                }
                            }
                            Err(e) => {
                                warn!("Failed to query capabilities for V4L2 device {}: {}", i, e);
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(io_err) = e.as_io_error() {
                            if io_err.kind() == std::io::ErrorKind::NotFound || io_err.kind() == std::io::ErrorKind::NoSuchDevice {
                                break;
                            }
                        }
                        warn!("Failed to open V4L2 device {}: {}", i, e);
                    }
                }
            }
            linux_names
        }
        #[cfg(not(target_os = "linux"))]
        {
            Vec::new()
        }
    };

    let mut avm_array_elements = Vec::with_capacity(names_vec.len());
    for name_val in names_vec {
        avm_array_elements.push(Value::String(name_val));
    }
    let array = ArrayObject::from_storage(activation.context.gc_context, activation.avm2().prototypes().array, avm_array_elements);
    Ok(Value::Object(array.into()))
}

/// Implements `flash.media.Camera.getCamera` static method
pub fn get_camera<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    #[cfg(target_os = "linux")]
    {
        let name_arg: Option<AvmString<'gc>> = args.get(0).and_then(|v| v.as_string().ok());
        let mut selected_device_index: Option<u32> = None;

        for i in 0..10 { // Check first 10 devices
            match Device::new(i) {
                Ok(device) => {
                    match device.query_caps() {
                        Ok(caps) => {
                            if caps.capabilities.contains(CapFlags::VIDEO_CAPTURE) {
                                let card_name_bytes: Vec<u8> = caps.card.iter().take_while(|&&c| c != 0).cloned().collect();
                                let card_name_str = String::from_utf8_lossy(&card_name_bytes).into_owned();

                                if let Some(target_name_avm) = name_arg {
                                    let target_name_rust = target_name_avm.to_utf8_lossy();
                                    if target_name_rust == card_name_str {
                                        selected_device_index = Some(i);
                                        break;
                                    }
                                } else {
                                    // No name argument, select the first available video capture device
                                    selected_device_index = Some(i);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("getCamera: Failed to query capabilities for V4L2 device {}: {}", i, e);
                        }
                    }
                }
                Err(e) => {
                    if let Some(io_err) = e.as_io_error() {
                        if io_err.kind() == std::io::ErrorKind::NotFound || io_err.kind() == std::io::ErrorKind::NoSuchDevice {
                            break; // No more devices
                        }
                    }
                    warn!("getCamera: Failed to open V4L2 device {}: {}", i, e);
                }
            }
        }

        if let Some(idx) = selected_device_index {
            info!("Selected V4L2 device index {} for new Camera instance.", idx);

            let camera_class = activation.avm2().classes().camera;
            match camera_class.construct(activation, &[]) {
                Ok(as_camera_obj) => {
                    info!("Successfully constructed AS Camera instance. Device index {} needs to be stored.", idx);
                    return Ok(as_camera_obj.into());
                }
                Err(e) => {
                    warn!("Failed to construct AS Camera instance: {}", e);
                    return Ok(Value::Null);
                }
            }
        } else {
            return Ok(Value::Null);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        return Ok(Value::Null);
    }
}


/// Implements `flash.media.Camera.isSupported` static getter
pub fn is_supported<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    #[cfg(target_os = "linux")]
    {
        for i in 0..10 { // Check first 10 devices
            match Device::new(i) {
                Ok(device) => {
                    match device.query_caps() {
                        Ok(caps) => {
                            if caps.capabilities.contains(CapFlags::VIDEO_CAPTURE) {
                                // Found at least one video capture device
                                return Ok(Value::Boolean(true));
                            }
                        }
                        Err(e) => {
                            warn!("isSupported: Failed to query capabilities for V4L2 device {}: {}", i, e);
                            // Continue to the next device even if querying caps fails for this one
                        }
                    }
                }
                Err(e) => {
                    if let Some(io_err) = e.as_io_error() {
                        if io_err.kind() == std::io::ErrorKind::NotFound || io_err.kind() == std::io::ErrorKind::NoSuchDevice {
                            // No more devices to check at or after this index
                            break;
                        }
                    }
                    warn!("isSupported: Failed to open V4L2 device {}: {}", i, e);
                    // If opening fails for a reason other than NotFound/NoSuchDevice,
                    // it might be a permission issue or device busy. We can continue to check other devices.
                }
            }
        }
        // If loop completes without returning true, no suitable device was found
        Ok(Value::Boolean(false))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Value::Boolean(false))
    }
}

pub fn create_class<'gc>(activation: &mut Activation<'_, 'gc>) -> ClassObject<'gc> {
    let mc = activation.context.gc_context;
    let class = Class::new(
        QName::new(Namespace::package("flash.media"), "Camera"),
        Some(activation.avm2().classes().eventdispatcher),
        Method::from_builtin(camera_constructor, "<Camera constructor>", mc),
        mc,
    );

    let scope = activation.create_scopechain();
    let class_object = ClassObject::from_class(activation, class, Some(scope));

    // Bind static getter `names`
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(), "names"),
        Method::from_builtin_static_getter(get_camera_names, "names", mc).into(),
        activation,
    );

    // Bind static method `getCamera`
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(), "getCamera"),
        Method::from_builtin(get_camera, "getCamera", mc).into(),
        activation,
    );

    // Bind static getter `isSupported`
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(), "isSupported"),
        Method::from_builtin_static_getter(is_supported, "isSupported", mc).into(),
        activation,
    );

    class_object
}
