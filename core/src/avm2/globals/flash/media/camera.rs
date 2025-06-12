//! flash.media.Camera native implementations

use crate::avm2::activation::Activation;
use crate::avm2::error::Error;
use crate::avm2::object::{ArrayObject, TObject, ClassObject}; // PrototypeObject removed
use crate::avm2::class::PrototypeObject; // PrototypeObject added here
use crate::avm2::value::Value;
use crate::avm2::method::Method;
use crate::avm2::qname::{Namespace, QName};
use crate::avm2::string::AvmString;
use crate::avm2::api_version::ApiVersion; // Needed for Namespace::package

#[cfg(target_os = "linux")]
use v4l::{Device, capability::Flags as CapFlags};
#[cfg(target_os = "linux")]
use v4l::error::Error as V4lError;
#[cfg(target_os = "linux")]
use tracing::{warn, info};


/// Placeholder for the Camera constructor (instance allocator)
pub fn camera_constructor<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>, // This is the TObject instance
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    // TODO: Actual instance initialization if needed.
    // For now, EventDispatcher's constructor does most of the work.
    // If CameraObject needs specific native data, initialize it here.
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
                            Err(e) => {
                                warn!("get_camera_names: Failed to query capabilities for V4L2 device {}: {}", i, e);
                            }
                        }
                    }
                    Err(V4lError::NotFound) => break,
                    Err(V4lError::Io(io_err)) => {
                        if io_err.kind() == std::io::ErrorKind::NotFound {
                            break;
                        } else if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                            warn!("get_camera_names: Permission denied opening V4L2 device {}: {}", i, io_err);
                        } else {
                            warn!("get_camera_names: I/O error opening V4L2 device {}: {}", i, io_err);
                        }
                    }
                    Err(e) => {
                        warn!("get_camera_names: Non-I/O error opening V4L2 device {}: {}", i, e);
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
    // Corrected ArrayObject creation
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
        // Corrected string coercion
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
                        Err(e) => {
                            warn!("getCamera: Failed to query capabilities for V4L2 device {}: {}", i, e);
                        }
                    }
                }
                Err(V4lError::NotFound) => break,
                Err(V4lError::Io(io_err)) => {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        break;
                    } else if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                        warn!("getCamera: Permission denied opening V4L2 device {}: {}", i, io_err);
                    } else {
                        warn!("getCamera: I/O error opening V4L2 device {}: {}", i, io_err);
                    }
                }
                Err(e) => {
                    warn!("getCamera: Non-I/O error opening V4L2 device {}: {}", i, e);
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
                                return Ok(true.into()); // Corrected Value construction
                            }
                        }
                        Err(e) => {
                            warn!("isSupported: Failed to query capabilities for V4L2 device {}: {}", i, e);
                        }
                    }
                }
                Err(V4lError::NotFound) => break,
                Err(V4lError::Io(io_err)) => {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        break;
                    } else if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                        warn!("isSupported: Permission denied opening V4L2 device {}: {}", i, io_err);
                    } else {
                        warn!("isSupported: I/O error opening V4L2 device {}: {}", i, io_err);
                    }
                }
                Err(e) => {
                    warn!("isSupported: Non-I/O error opening V4L2 device {}: {}", i, e);
                }
            }
        }
        Ok(false.into()) // Corrected Value construction
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(false.into()) // Corrected Value construction
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
        camera_constructor, // NativeMethodImpl for instance allocator
        None, // instance_call_handler
        None, // class_call_handler
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

    // Bind static getter `names`
    let names_method = Method::from_builtin_static_getter_and_params(get_camera_names, "names", Vec::new(), mc, None);
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(activation.context.gc_context), AvmString::new_utf8(mc, "names")), // Ensure public_namespace gets mc
        names_method.into(),
        activation,
    )?;

    // Bind static method `getCamera`
    let get_camera_method = Method::from_builtin_and_params(get_camera, "getCamera", Vec::new(), mc, None);
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(activation.context.gc_context), AvmString::new_utf8(mc, "getCamera")),
        get_camera_method.into(),
        activation,
    )?;

    // Bind static getter `isSupported`
    let is_supported_method = Method::from_builtin_static_getter_and_params(is_supported, "isSupported", Vec::new(), mc, None);
    class_object.define_class_trait(
        mc,
        QName::new(Namespace::public_namespace(activation.context.gc_context), AvmString::new_utf8(mc, "isSupported")),
        is_supported_method.into(),
        activation,
    )?;

    Ok(class_object)
}
