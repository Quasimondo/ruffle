use ruffle_render::pixel_bender::{PixelBenderType, PixelBenderTypeOpcode};

use crate::{
    avm2::{Activation, ArrayObject, ArrayStorage, Error, TObject, Value},
    ecma_conversions::f64_to_wrapping_i32,
    string::AvmString,
};

/// This trait provides methods for converting between PixelBender types and AVM2 values.
/// PixelBender is a domain-specific language for image processing, and its types need to be
/// representable in the AVM2 (ActionScript Virtual Machine 2) environment.
pub trait PixelBenderTypeExt {
    /// Converts an AVM2 `Value` into a `PixelBenderType`.
    ///
    /// This method takes an AVM2 `Value`, which can represent various ActionScript types like
    /// Number, int, String, or Array, and attempts to convert it into a specific `PixelBenderType`
    /// based on the provided `kind` (PixelBenderTypeOpcode).
    ///
    /// For example, if `kind` is `TFloat`, this method will try to coerce the AVM2 `value`
    /// into a floating-point number and return it as `PixelBenderType::TFloat`. If the `value`
    /// is an Array, it will be converted into vector or matrix types (e.g., TFloat2, TFloat3x3)
    /// based on the `kind`.
    ///
    /// # Arguments
    ///
    /// * `activation`: A mutable reference to the AVM2 `Activation` environment.
    /// * `value`: The AVM2 `Value` to convert.
    /// * `kind`: The `PixelBenderTypeOpcode` indicating the target PixelBender type.
    ///
    /// # Returns
    ///
    /// A `Result` containing the converted `PixelBenderType` on success, or an `Error`
    /// if the conversion is not possible or an unexpected type is encountered.
    fn from_avm2_value<'gc>(
        activation: &mut Activation<'_, 'gc>,
        value: Value<'gc>,
        kind: &PixelBenderTypeOpcode,
    ) -> Result<Self, Error<'gc>>
    where
        Self: Sized;

    /// Converts a `PixelBenderType` into an AVM2 `Value`.
    ///
    /// This method takes a `PixelBenderType` and converts it into an AVM2 `Value` that can be
    /// used within the ActionScript environment.
    ///
    /// For scalar types like `TString`, `TInt`, or `TFloat`, it returns a corresponding AVM2
    /// String, int, or Number. For vector or matrix types (e.g., `TFloat2`, `TFloat3x3`),
    /// it returns an AVM2 Array containing the elements.
    ///
    /// The `tint_as_int` parameter controls how `PixelBenderType::TInt` is represented. If `true`,
    /// it's returned as an AVM2 `int`. Otherwise, it's wrapped in an Array, similar to other
    /// vector types. Floating-point numbers with no fractional part may be converted to AVM2 `int`
    /// for compatibility with Flash behavior.
    ///
    /// # Arguments
    ///
    /// * `activation`: A mutable reference to the AVM2 `Activation` environment.
    /// * `tint_as_int`: A boolean flag that, if true, causes `PixelBenderType::TInt` to be
    ///   returned as a direct AVM2 `int` value. If false, `TInt` is returned as an Array
    ///   containing a single integer.
    ///
    /// # Returns
    ///
    /// A `Result` containing the converted AVM2 `Value` on success, or an `Error` if
    /// an issue occurs during conversion.
    fn as_avm2_value<'gc>(
        &self,
        activation: &mut Activation<'_, 'gc>,
        tint_as_int: bool,
    ) -> Result<Value<'gc>, Error<'gc>>;
}

/// Implements the `PixelBenderTypeExt` trait for the `PixelBenderType` enum.
/// This allows direct conversion between `PixelBenderType` variants and AVM2 `Value`s.
impl PixelBenderTypeExt for PixelBenderType {
    /// Converts an AVM2 `Value` to a specific `PixelBenderType` variant.
    ///
    /// This implementation handles the following conversions:
    /// - AVM2 `String` to `PixelBenderType::TString`.
    /// - AVM2 `Number` to `PixelBenderType::TFloat`.
    /// - AVM2 `Integer` to `PixelBenderType::TInt`.
    /// - AVM2 `Array` to various `PixelBenderType` vector and matrix types (e.g., `TFloat2`, `TInt4`, `TFloat3x3`),
    ///   based on the `kind` parameter. The elements of the array are coerced to numbers (for float types)
    ///   or integers (for int types).
    ///
    /// Panics if an unexpected AVM2 `value` type is provided for the given `kind`, or if an AVM2 `Object`
    /// that is not an `Array` is encountered when a vector or matrix type is expected.
    /// It also panics if an array has holes or if the number of elements in the array does not match
    /// the expected size for the given `kind`.
    fn from_avm2_value<'gc>(
        activation: &mut Activation<'_, 'gc>,
        value: Value<'gc>,
        kind: &PixelBenderTypeOpcode,
    ) -> Result<Self, Error<'gc>>
    where
        Self: Sized,
    {
        let is_float = matches!(
            kind,
            PixelBenderTypeOpcode::TFloat
                | PixelBenderTypeOpcode::TFloat2
                | PixelBenderTypeOpcode::TFloat3
                | PixelBenderTypeOpcode::TFloat4
                | PixelBenderTypeOpcode::TFloat2x2
                | PixelBenderTypeOpcode::TFloat3x3
                | PixelBenderTypeOpcode::TFloat4x4
        );

        match value {
            Value::String(s) => Ok(PixelBenderType::TString(s.to_string())),
            Value::Number(n) => Ok(PixelBenderType::TFloat(n as f32)),
            Value::Integer(i) => Ok(PixelBenderType::TInt(i as i16)),
            Value::Object(o) => {
                if let Some(array) = o.as_array_storage() {
                    if is_float {
                        let mut vals = array.iter().map(|val| {
                            val.expect("Array with hole")
                                .coerce_to_number(activation)
                                .unwrap() as f32
                        });
                        match kind {
                            PixelBenderTypeOpcode::TFloat => {
                                Ok(PixelBenderType::TFloat(vals.next().unwrap()))
                            }
                            PixelBenderTypeOpcode::TFloat2 => Ok(PixelBenderType::TFloat2(
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TFloat3 => Ok(PixelBenderType::TFloat3(
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TFloat4 => Ok(PixelBenderType::TFloat4(
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TFloat2x2 => Ok(PixelBenderType::TFloat2x2(
                                vals.collect::<Vec<_>>().try_into().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TFloat3x3 => Ok(PixelBenderType::TFloat3x3(
                                vals.collect::<Vec<_>>().try_into().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TFloat4x4 => Ok(PixelBenderType::TFloat4x4(
                                vals.collect::<Vec<_>>().try_into().unwrap(),
                            )),
                            _ => unreachable!("Unexpected float kind {kind:?}"),
                        }
                    } else {
                        let mut vals = array.iter().map(|val| {
                            val.expect("Array with hole")
                                .coerce_to_i32(activation)
                                .unwrap() as i16
                        });
                        match kind {
                            PixelBenderTypeOpcode::TInt => {
                                Ok(PixelBenderType::TInt(vals.next().unwrap()))
                            }
                            PixelBenderTypeOpcode::TInt2 => Ok(PixelBenderType::TInt2(
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TInt3 => Ok(PixelBenderType::TInt3(
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                            )),
                            PixelBenderTypeOpcode::TInt4 => Ok(PixelBenderType::TInt4(
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                                vals.next().unwrap(),
                            )),
                            _ => unreachable!("Unexpected int kind {kind:?}"),
                        }
                    }
                } else {
                    panic!("Unexpected object {o:?}")
                }
            }
            _ => panic!("Unexpected value {value:?}"),
        }
    }

    /// Converts a `PixelBenderType` variant to an AVM2 `Value`.
    ///
    /// This implementation handles the following conversions:
    /// - `PixelBenderType::TString` to AVM2 `String`.
    /// - `PixelBenderType::TInt`:
    ///     - If `tint_as_int` is `true`, converts to an AVM2 `Integer`.
    ///     - If `tint_as_int` is `false`, converts to an AVM2 `Array` containing a single integer.
    /// - `PixelBenderType::TFloat` to an AVM2 `Number`. If the float has no fractional part,
    ///   it's converted to an AVM2 `Integer` (emulating Flash behavior).
    /// - Vector types (`TFloat2`, `TFloat3`, `TFloat4`, `TInt2`, `TInt3`, `TInt4`) to an AVM2 `Array`
    ///   of numbers or integers. Floats with no fractional parts are converted to integers.
    /// - Matrix types (`TFloat2x2`, `TFloat3x3`, `TFloat4x4`) to an AVM2 `Array` of numbers,
    ///   representing the matrix elements in row-major order. Floats with no fractional parts
    ///   are converted to integers.
    ///
    /// The conversion of floats to integers when there's no fractional part is done to match
    /// the behavior observed in the Flash Player.
    fn as_avm2_value<'gc>(
        &self,
        activation: &mut Activation<'_, 'gc>,
        tint_as_int: bool,
    ) -> Result<Value<'gc>, Error<'gc>> {
        // Flash appears to use a uint/int if the float has no fractional part
        let cv = |f: &f32| -> Value<'gc> {
            if f.fract() == 0.0 {
                f64_to_wrapping_i32(*f as f64).into()
            } else {
                (*f).into()
            }
        };
        let vals: Vec<Value<'gc>> = match self {
            PixelBenderType::TString(string) => {
                return Ok(AvmString::new_utf8(activation.gc(), string).into());
            }
            PixelBenderType::TInt(i) => {
                if tint_as_int {
                    return Ok((*i).into());
                } else {
                    vec![(*i).into()]
                }
            }
            PixelBenderType::TFloat(f) => vec![cv(f)],
            PixelBenderType::TFloat2(f1, f2) => vec![cv(f1), cv(f2)],
            PixelBenderType::TFloat3(f1, f2, f3) => vec![cv(f1), cv(f2), cv(f3)],
            PixelBenderType::TFloat4(f1, f2, f3, f4) => vec![cv(f1), cv(f2), cv(f3), cv(f4)],
            PixelBenderType::TFloat2x2(floats) => floats.iter().map(cv).collect(),
            PixelBenderType::TFloat3x3(floats) => floats.iter().map(cv).collect(),
            PixelBenderType::TFloat4x4(floats) => floats.iter().map(cv).collect(),
            PixelBenderType::TInt2(i1, i2) => vec![(*i1).into(), (*i2).into()],
            PixelBenderType::TInt3(i1, i2, i3) => vec![(*i1).into(), (*i2).into(), (*i3).into()],
            PixelBenderType::TInt4(i1, i2, i3, i4) => {
                vec![(*i1).into(), (*i2).into(), (*i3).into(), (*i4).into()]
            }
        };
        let storage = ArrayStorage::from_args(&vals);
        Ok(ArrayObject::from_storage(activation, storage).into())
    }
}
