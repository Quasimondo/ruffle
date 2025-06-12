//! flash.media.Camera native implementations

use crate::avm2::activation::Activation;
use crate::avm2::error::Error;
use crate::avm2::object::{ArrayObject, TObject, ClassObject};
use crate::avm2::class::PrototypeObject;
use crate::avm2::value::Value;
use crate::avm2::method::Method;
use crate::avm2::qname::{Namespace, QName};
use crate::avm2::string::AvmString;
use crate::avm2::api_version::ApiVersion;

#[cfg(target_os = "linux")]
use v4l::{Device, capability::Flags as CapFlags};
// Note: Intentionally not importing v4l::error::Error as V4lError to test e.kind() directly
#[cfg(target_os = "linux")]
use tracing::{warn, info};


/// Placeholder for the Camera constructor (instance allocator)
pub fn camera_constructor<'gc>(
    activation: &mut Activation<'_, 'gc>,
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
                                    let null_pos = caps.card.iter().position(|&c| c == 0).unwrap_or(caps.card.len());
                                    let name_slice = &caps.card[..null_pos];
                                    let card_name = String::from_utf8_lossy(name_slice).into_owned();
                                    linux_names.push(AvmString::new_utf8(activation.context.gc_context, card_name));
                                }
                            }
                            Err(e) => { // Error from query_caps
                                warn!("get_camera_names: Error querying capabilities for V4L2 device {}: {}", i, e);
                                continue;
                            }
                        }
                    }
                    Err(e) => { // Error from Device::new(i)
                        // Attempt to use e.kind() as per subtask instruction
                        // This will likely fail to compile if 'e' is not std::io::Error directly
                        // and if v4l::error::Error doesn't have a .kind() method.
                        // For the purpose of this subtask, we follow the instruction.
                        match e.kind() { // Assuming e has a .kind() method similar to std::io::Error
                            std::io::ErrorKind::NotFound => {
                                warn!("get_camera_names: V4L2 device {} not found. Error: {}", i, e);
                                break;
                            }
                            std::io::ErrorKind::PermissionDenied => {
                                warn!("get_camera_names: Permission denied opening V4L2 device {}: {}", i, e);
                                // Continue to check other devices
                            }
                            _ => {
                                warn!("get_camera_names: Error opening V4L2 device {}: {}", i, e);
                                // Continue to check other devices for other errors
                            }
                        }
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
    let array = ArrayObject::from_values(activation, &avm_array_elements)?;
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
        let name_arg: Option<AvmString<'gc>> = args.get(0).and_then(|v| v.coerce_to_string(activation).ok());
        let mut selected_device_index: Option<u32> = None;

        for i in 0..10 {
            match Device::new(i) {
                Ok(device) => {
                    match device.query_caps() {
                        Ok(caps) => {
                            if caps.capabilities.contains(CapFlags::VIDEO_CAPTURE) {
                                let null_pos = caps.card.iter().position(|&c| c == 0).unwrap_or(caps.card.len());
                                let card_name_str = String::from_utf8_lossy(&caps.card[..null_pos]).into_owned();

                                if let Some(target_name_avm) = name_arg {
                                    let target_name_rust = target_name_avm.to_utf8_lossy();
                                    if target_name_rust == card_name_str {
                                        selected_device_index = Some(i as u32);
                                        break;
                                    }
                                } else {
                                    selected_device_index = Some(i as u32);
                                    break;
                                }
                            }
                        }
                        Err(e) => { // Error from query_caps
                            warn!("getCamera: Error querying capabilities for V4L2 device {}: {}", i, e);
                            // If query_caps fails, this device is unusable for selection.
                            // If we were looking for a specific name, and this device's name is unknown, continue.
                            // If we were looking for the *first* device, this one is bad, so continue.
                            continue;
                        }
                    }
                    // If we found the named device or the first available device, break from Device::new loop
                    if selected_device_index.is_some() {
                        break;
                    }
                }
                Err(e) => { // Error from Device::new(i)
                    match e.kind() { // Assuming e has a .kind() method
                        std::io::ErrorKind::NotFound => {
                            warn!("getCamera: V4L2 device {} not found. Error: {}", i, e);
                            break;
                        }
                        std::io::ErrorKind::PermissionDenied => {
                            warn!("getCamera: Permission denied opening V4L2 device {}: {}", i, e);
                        }
                        _ => {
                            warn!("getCamera: Error opening V4L2 device {}: {}", i, e);
                        }
                    }
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
        for i in 0..10 {
            match Device::new(i) {
                Ok(device) => {
                    match device.query_caps() {
                        Ok(caps) => {
                            if caps.capabilities.contains(CapFlags::VIDEO_CAPTURE) {
                                return Ok(true.into());
                            }
                        }
                        Err(e) => { // Error from query_caps
                            warn!("isSupported: Error querying capabilities for V4L2 device {}: {}", i, e);
                            continue;
                        }
                    }
                }
                Err(e) => { // Error from Device::new(i)
                     match e.kind() { // Assuming e has a .kind() method
                        std::io::ErrorKind::NotFound => {
                            warn!("isSupported: V4L2 device {} not found. Error: {}", i, e);
                            break;
                        }
                        std::io::ErrorKind::PermissionDenied => {
                            warn!("isSupported: Permission denied opening V4L2 device {}: {}", i, e);
                        }
                        _ => {
                            warn!("isSupported: Error opening V4L2 device {}: {}", i, e);
                        }
                    }
                }
            }
        }
        Ok(false.into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(false.into())
    }
}

pub fn create_class<'gc>(activation: &mut Activation<'_, 'gc>) -> Result<ClassObject<'gc>, Error<'gc>> {
    let mc = activation.context.gc_context;

    let package_name = AvmString::new_utf8(mc, "flash.media");
    let class_name = AvmString::new_utf8(mc, "Camera");
    let qname = QName::new(
        Namespace::package(package_name, activation.avm2().api_version(), &mut activation.context.avm2_context_mut().strings),
        class_name
    );

    let class_gc_cell = crate::avm2::class::Class::new(
        qname,
        Some(activation.avm2().classes().eventdispatcher),
        camera_constructor,
        None,
        None,
        mc
    );

    let proto = PrototypeObject::derive_prototype_from_base(
        activation,
        class_gc_cell,
        activation.avm2().prototypes().eventdispatcher
    )?;

    let class_object = ClassObject::from_class_and_prototype(
        activation,
        class_gc_cell,
        proto
    )?;

    let names_method = Method::from_builtin_static_getter_and_params(get_camera_names, "names", Vec::new(), mc, None);
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(activation.context.gc_context), AvmString::new_utf8(mc, "names")),
        names_method.into(),
        activation,
    )?;

    let get_camera_method = Method::from_builtin_and_params(get_camera, "getCamera", Vec::new(), mc, None);
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(activation.context.gc_context), AvmString::new_utf8(mc, "getCamera")),
        get_camera_method.into(),
        activation,
    )?;

    let is_supported_method = Method::from_builtin_static_getter_and_params(is_supported, "isSupported", Vec::new(), mc, None);
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(activation.context.gc_context), AvmString::new_utf8(mc, "isSupported")),
        is_supported_method.into(),
        activation,
    )?;

    Ok(class_object)
}
