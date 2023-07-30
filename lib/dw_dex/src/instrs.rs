//! Dalvik bytecode instructions definitions.

use crate::code::{CallSiteIdItem, MethodHandle, MethodHandleItem};
use crate::errors::DexResult;
use crate::fields::FieldIdItem;
use crate::methods::MethodIdItem;
use crate::registers::{Reg, RegList, RegRange};
use crate::strings::StringIdItem;
use crate::types::{ProtoIdItem, Type, TypeIdItem};
use crate::values::EncodedValue;
use crate::{Addr, Dex, DexIndex, Index, PrettyPrint, WithDex};
use instruction_derive::Instruction;
use serde::ser::{self, SerializeStruct, Serializer};
use serde::Serialize;
use std::fmt;

pub trait Instruction {
    fn mnemonic(&self) -> &str;
    fn size(&self) -> usize;
    fn can_throw(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct LabeledInstr {
    pub(crate) addr: Addr,
    pub(crate) instr: Instr,
}

impl Instruction for LabeledInstr {
    #[inline]
    fn mnemonic(&self) -> &str {
        self.instr.mnemonic()
    }

    #[inline]
    fn size(&self) -> usize {
        self.instr.size()
    }

    #[inline]
    fn can_throw(&self) -> bool {
        self.instr.can_throw()
    }
}

impl LabeledInstr {
    #[inline]
    #[must_use]
    pub const fn addr(&self) -> Addr {
        self.addr
    }

    #[inline]
    #[must_use]
    pub const fn instr(&self) -> &Instr {
        &self.instr
    }

    #[inline]
    #[must_use]
    pub fn next_addr(&self) -> Addr {
        self.addr().offset(self.instr().size() as i32)
    }
}

impl<'a> Serialize for WithDex<'a, LabeledInstr> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let operands = collect_operands::<S>(self.data.instr(), self.dex)?;

        let mut state = serializer.serialize_struct("Instr", 3)?;
        state.serialize_field("address", &self.data.addr().0)?;
        state.serialize_field("mnemonic", &self.data.mnemonic())?;
        state.serialize_field("operands", &operands)?;
        state.end()
    }
}

#[derive(Debug, Clone, Instruction)]
pub enum Instr {
    /// Waste cycles.
    #[instruction(mnemonic = "nop", format = "10x")]
    Nop,

    /// Move the contents of one non-object register to another.
    #[instruction(mnemonic = "move", format = "12x")]
    Move(Reg, Reg),

    /// Move the contents of one non-object register to another.
    #[instruction(mnemonic = "move/from16", format = "22x")]
    MoveFrom16(Reg, Reg),

    /// Move the contents of one non-object register to another.
    #[instruction(mnemonic = "move/16", format = "32x")]
    Move16(Reg, Reg),

    /// Move the contents of one register-pair to another.
    #[instruction(mnemonic = "move-wide", format = "12x")]
    MoveWide(Reg, Reg),

    /// Move the contents of one register-pair to another.
    #[instruction(mnemonic = "move-wide/from16", format = "22x")]
    MoveWideFrom16(Reg, Reg),

    /// Move the contents of one register-pair to another.
    #[instruction(mnemonic = "move-wide/16", format = "32x")]
    MoveWide16(Reg, Reg),

    /// Move the contents of one object-bearing register to another.
    #[instruction(mnemonic = "move-object", format = "12x")]
    MoveObject(Reg, Reg),

    /// Move the contents of one object-bearing register to another.
    #[instruction(mnemonic = "move-object/from16", format = "22x")]
    MoveObjectFrom16(Reg, Reg),

    /// Move the contents of one object-bearing register to another.
    #[instruction(mnemonic = "move-object/16", format = "32x")]
    MoveObject16(Reg, Reg),

    /// Move the single-word non-object result of the most recent invoke-kind into
    /// the indicated register.
    ///
    /// This must be done as the instruction immediately after an invoke-kind whose
    /// (single-word, non-object) result is not to be ignored; anywhere else is invalid.
    #[instruction(mnemonic = "move-result", format = "11x")]
    MoveResult(Reg),

    /// Move the double-word result of the most recent invoke-kind into the indicated register.
    ///
    /// This must be done as the instruction immediately after an invoke-kind whose
    /// (double-word) result is not to be ignored; anywhere else is invalid.
    #[instruction(mnemonic = "move-result-wide", format = "11x")]
    MoveResultWide(Reg),

    /// Move the object result of the most invoke-kind into the indicated register.
    ///
    /// This must be done as the instruction immediately after an invoke-kind or filled-new-array
    /// whose (object) result is not to be ignored; anywhere else is invalid.
    #[instruction(mnemonic = "move-result-object", format = "11x")]
    MoveResultObject(Reg),

    /// Save a just-caught exception into the given register.
    ///
    /// This must be the first instruction of any exception handler whose caught exception is
    /// not to be ignored, and this instruction must only ever occur as the first instruction of
    /// an exception handler; anywhere else is invalid.
    #[instruction(mnemonic = "move-exception", format = "11x")]
    MoveException(Reg),

    /// Return from a void method.
    #[instruction(mnemonic = "return-void", format = "10x")]
    ReturnVoid,

    /// Return from a single-width (32-bit) non-object value-returning method.
    #[instruction(mnemonic = "return", format = "11x")]
    Return(Reg),

    /// Return from a double-width (64-bit) value-returning method.
    #[instruction(mnemonic = "return-wide", format = "11x")]
    ReturnWide(Reg),

    /// Return from an object-returning method.
    #[instruction(mnemonic = "return-object", format = "11x")]
    ReturnObject(Reg),

    /// Move the given literal value (sign-extended to 32 bits) into the specified register.
    #[instruction(mnemonic = "const/4", format = "11n")]
    Const4(Reg, i8),

    /// Move the given literal value (sign-extended to 32 bits) into the specified register.
    #[instruction(mnemonic = "const/16", format = "21s")]
    Const16(Reg, i16),

    /// Move the given literal value into the specified register.
    #[instruction(mnemonic = "const", format = "31i")]
    Const(Reg, i32),

    /// Move the given literal value (right-zero-extended to 32 bits) into the specified register.
    #[instruction(mnemonic = "const/high16", format = "21h")]
    ConstHigh16(Reg, i16),

    /// Move the given literal value (sign-extended to 64 bits) into the specified register-pair.
    #[instruction(mnemonic = "const-wide/16", format = "21s")]
    ConstWide16(Reg, i16),

    /// Move the given literal value (sign-extended to 64 bits) into the specified register-pair.
    #[instruction(mnemonic = "const-wide/32", format = "31i")]
    ConstWide32(Reg, i32),

    /// Move the given literal value into the specified register-pair.
    #[instruction(mnemonic = "const-wide", format = "51l")]
    ConstWide(Reg, i64),

    /// Move the given literal value (right-zero-extended to 64 bits) into the specified register-pair.
    #[instruction(mnemonic = "const-wide/high16", format = "21h")]
    ConstWideHigh16(Reg, i16),

    /// Move a reference to the string specified by the given index into the specified register.
    #[instruction(mnemonic = "const-string", format = "21c")]
    ConstString(Reg, Index<StringIdItem>),

    /// Move a reference to the string specified by the given index into the specified register.
    #[instruction(mnemonic = "const-string/jumbo", format = "31c")]
    ConstStringJumbo(Reg, Index<StringIdItem>),

    /// Move a reference to the class specified by the given index into the specified register.
    /// In the case where the indicated type is primitive, this will store a reference to the
    /// primitive type's degenerate class.
    #[instruction(mnemonic = "const-class", format = "21c")]
    ConstClass(Reg, Index<TypeIdItem>),

    /// Acquire the monitor for the indicated object.
    #[instruction(mnemonic = "monitor-enter", format = "11x")]
    MonitorEnter(Reg),

    /// Release the monitor for the indicated object.
    #[instruction(mnemonic = "monitor-exit", format = "11x", can_throw)]
    MonitorExit(Reg),

    /// Throw a `ClassCastException` if the reference in the given register cannot be cast to the indicated type.
    #[instruction(mnemonic = "check-cast", format = "21c", can_throw)]
    CheckCast(Reg, Index<TypeIdItem>),

    /// Store in the given destination register 1 if the indicated reference is an instance of the given type,
    /// or 0 if not.
    #[instruction(mnemonic = "instance-of", format = "22c")]
    InstanceOf(Reg, Reg, Index<TypeIdItem>),

    /// Store in the given destination register the length of the indicated array, in entries.
    #[instruction(mnemonic = "array-length", format = "12x", can_throw)]
    ArrayLength(Reg, Reg),

    /// Construct a new instance of the indicated type, storing a reference to it in the destination.
    /// The type must refer to a non-array class.
    #[instruction(mnemonic = "new-instance", format = "21c")]
    NewInstance(Reg, Index<TypeIdItem>),

    /// Construct a new array of the indicated type and size.
    /// The type must be an array type.
    #[instruction(mnemonic = "new-array", format = "22c")]
    NewArray(Reg, Reg, Index<TypeIdItem>),

    /// Construct an array of the given type and size, filling it with the supplied contents.
    /// The type must be an array type.
    /// The array's contents must be single-word (that is, no arrays of long or double,
    /// but reference types are acceptable).
    /// The constructed instance is stored as a "result" in the same way that the method
    /// invocation instructions store their results, so the constructed instance must be
    /// moved to a register with an immediately subsequent `move-result-object` instruction
    /// (if it is to be used).
    #[instruction(mnemonic = "filled-new-array", format = "35c", can_throw)]
    FilledNewArray(RegList, Index<TypeIdItem>),

    /// Construct an array of the given type and size, filling it with the supplied contents.
    /// Clarifications and restrictions are the same as filled-new-array, described above.
    #[instruction(mnemonic = "filled-new-array/range", format = "3rc", can_throw)]
    FilledNewArrayRange(RegRange, Index<TypeIdItem>),

    /// Fill the given array with the indicated data.
    /// The reference must be to an array of primitives, and the data table must match it in
    /// type and must contain no more elements than will fit in the array.
    /// That is, the array may be larger than the table, and if so, only the initial elements
    /// of the array are set, leaving the remainder alone.
    #[instruction(mnemonic = "fill-array-data", format = "31t", can_throw)]
    FillArrayData(Reg, i32),

    /// Throw the indicated exception.
    #[instruction(mnemonic = "throw", format = "11x", can_throw)]
    Throw(Reg),

    /// Unconditionally jump to the indicated instruction.
    #[instruction(mnemonic = "goto", format = "10t")]
    Goto(i8),

    /// Unconditionally jump to the indicated instruction.
    #[instruction(mnemonic = "goto/16", format = "20t")]
    Goto16(i16),

    /// Unconditionally jump to the indicated instruction.
    #[instruction(mnemonic = "goto/32", format = "30t")]
    Goto32(i32),

    /// Jump to a new instruction based on the value in the given register,
    /// using a table of offsets corresponding to each value in a particular integral range,
    /// or fall through to the next instruction if there is no match.
    #[instruction(mnemonic = "packed-switch", format = "31t")]
    PackedSwitch(Reg, i32),

    /// Jump to a new instruction based on the value in the given register,
    /// using an ordered table of value-offset pairs,
    /// or fall through to the next instruction if there is no match.
    #[instruction(mnemonic = "sparse-switch", format = "31t")]
    SparseSwitch(Reg, i32),

    #[instruction(mnemonic = "cmpl-float", format = "23x")]
    CmplFloat(Reg, Reg, Reg),
    #[instruction(mnemonic = "cmpg-float", format = "23x")]
    CmpgFloat(Reg, Reg, Reg),
    #[instruction(mnemonic = "cmpl-double", format = "23x")]
    CmplDouble(Reg, Reg, Reg),
    #[instruction(mnemonic = "cmpg-double", format = "23x")]
    CmpgDouble(Reg, Reg, Reg),
    #[instruction(mnemonic = "cmp-long", format = "23x")]
    CmpLong(Reg, Reg, Reg),

    /// Branch to the given destination if the given two registers' values compare as specified.
    #[instruction(mnemonic = "if-eq", format = "22t")]
    IfEq(Reg, Reg, i16),

    /// Branch to the given destination if the given two registers' values compare as specified.
    #[instruction(mnemonic = "if-ne", format = "22t")]
    IfNe(Reg, Reg, i16),

    /// Branch to the given destination if the given two registers' values compare as specified.
    #[instruction(mnemonic = "if-lt", format = "22t")]
    IfLt(Reg, Reg, i16),

    /// Branch to the given destination if the given two registers' values compare as specified.
    #[instruction(mnemonic = "if-ge", format = "22t")]
    IfGe(Reg, Reg, i16),

    /// Branch to the given destination if the given two registers' values compare as specified.
    #[instruction(mnemonic = "if-gt", format = "22t")]
    IfGt(Reg, Reg, i16),

    /// Branch to the given destination if the given two registers' values compare as specified.
    #[instruction(mnemonic = "if-le", format = "22t")]
    IfLe(Reg, Reg, i16),

    /// Branch to the given destination if the given register's value compares with 0 as specified.
    #[instruction(mnemonic = "if-eqz", format = "21t")]
    IfEqz(Reg, i16),

    /// Branch to the given destination if the given register's value compares with 0 as specified.
    #[instruction(mnemonic = "if-nez", format = "21t")]
    IfNez(Reg, i16),

    /// Branch to the given destination if the given register's value compares with 0 as specified.
    #[instruction(mnemonic = "if-ltz", format = "21t")]
    IfLtz(Reg, i16),

    /// Branch to the given destination if the given register's value compares with 0 as specified.
    #[instruction(mnemonic = "if-gez", format = "21t")]
    IfGez(Reg, i16),

    /// Branch to the given destination if the given register's value compares with 0 as specified.
    #[instruction(mnemonic = "if-gtz", format = "21t")]
    IfGtz(Reg, i16),

    /// Branch to the given destination if the given register's value compares with 0 as specified.
    #[instruction(mnemonic = "if-lez", format = "21t")]
    IfLez(Reg, i16),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget", format = "23x", can_throw)]
    Aget(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget-wide", format = "23x", can_throw)]
    AgetWide(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget-object", format = "23x", can_throw)]
    AgetObject(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget-boolean", format = "23x", can_throw)]
    AgetBoolean(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget-byte", format = "23x", can_throw)]
    AgetByte(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget-char", format = "23x", can_throw)]
    AgetChar(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aget-short", format = "23x", can_throw)]
    AgetShort(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput", format = "23x", can_throw)]
    Aput(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput-wide", format = "23x", can_throw)]
    AputWide(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput-object", format = "23x", can_throw)]
    AputObject(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput-boolean", format = "23x", can_throw)]
    AputBoolean(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput-byte", format = "23x", can_throw)]
    AputByte(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput-char", format = "23x", can_throw)]
    AputChar(Reg, Reg, Reg),

    /// Perform the identified array operation at the identified index of the given array,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "aput-short", format = "23x", can_throw)]
    AputShort(Reg, Reg, Reg),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget", format = "22c")]
    Iget(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget-wide", format = "22c")]
    IgetWide(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget-object", format = "22c")]
    IgetObject(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget-boolean", format = "22c")]
    IgetBoolean(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget-byte", format = "22c")]
    IgetByte(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget-char", format = "22c")]
    IgetChar(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iget-short", format = "22c")]
    IgetShort(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput", format = "22c")]
    Iput(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput-wide", format = "22c")]
    IputWide(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput-object", format = "22c")]
    IputObject(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput-boolean", format = "22c")]
    IputBoolean(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput-byte", format = "22c")]
    IputByte(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput-char", format = "22c")]
    IputChar(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object instance field operation with the identified field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "iput-short", format = "22c")]
    IputShort(Reg, Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget", format = "21c")]
    Sget(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget-wide", format = "21c")]
    SgetWide(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget-object", format = "21c")]
    SgetObject(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget-boolean", format = "21c")]
    SgetBoolean(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget-byte", format = "21c")]
    SgetByte(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget-char", format = "21c")]
    SgetChar(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sget-short", format = "21c")]
    SgetShort(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput", format = "21c")]
    Sput(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput-wide", format = "21c")]
    SputWide(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput-object", format = "21c")]
    SputObject(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput-boolean", format = "21c")]
    SputBoolean(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput-byte", format = "21c")]
    SputByte(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput-char", format = "21c")]
    SputChar(Reg, Index<FieldIdItem>),

    /// Perform the identified object static field operation with the identified static field,
    /// loading or storing into the value register.
    #[instruction(mnemonic = "sput-short", format = "21c")]
    SputShort(Reg, Index<FieldIdItem>),

    #[instruction(mnemonic = "invoke-virtual", format = "35c", can_throw)]
    InvokeVirtual(RegList, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-super", format = "35c", can_throw)]
    InvokeSuper(RegList, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-direct", format = "35c", can_throw)]
    InvokeDirect(RegList, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-static", format = "35c", can_throw)]
    InvokeStatic(RegList, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-interface", format = "35c", can_throw)]
    InvokeInterface(RegList, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-virtual/range", format = "3rc", can_throw)]
    InvokeVirtualRange(RegRange, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-super/range", format = "3rc", can_throw)]
    InvokeSuperRange(RegRange, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-direct/range", format = "3rc", can_throw)]
    InvokeDirectRange(RegRange, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-static/range", format = "3rc", can_throw)]
    InvokeStaticRange(RegRange, Index<MethodIdItem>),
    #[instruction(mnemonic = "invoke-interface/range", format = "3rc", can_throw)]
    InvokeInterfaceRange(RegRange, Index<MethodIdItem>),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "neg-int", format = "12x")]
    NegInt(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "not-int", format = "12x")]
    NotInt(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "neg-long", format = "12x")]
    NegLong(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "not-long", format = "12x")]
    NotLong(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "neg-float", format = "12x")]
    NegFloat(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "neg-double", format = "12x")]
    NegDouble(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "int-to-long", format = "12x")]
    IntToLong(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "int-to-float", format = "12x")]
    IntToFloat(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "int-to-double", format = "12x")]
    IntToDouble(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "long-to-int", format = "12x")]
    LongToInt(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "long-to-float", format = "12x")]
    LongToFloat(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "long-to-double", format = "12x")]
    LongToDouble(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "float-to-int", format = "12x")]
    FloatToInt(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "float-to-long", format = "12x")]
    FloatToLong(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "float-to-double", format = "12x")]
    FloatToDouble(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "double-to-int", format = "12x")]
    DoubleToInt(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "double-to-long", format = "12x")]
    DoubleToLong(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "double-to-float", format = "12x")]
    DoubleToFloat(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "int-to-byte", format = "12x")]
    IntToByte(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "int-to-char", format = "12x")]
    IntToChar(Reg, Reg),

    /// Perform the identified unary operation on the source register,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "int-to-short", format = "12x")]
    IntToShort(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "add-int", format = "23x")]
    AddInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "sub-int", format = "23x")]
    SubInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "mul-int", format = "23x")]
    MulInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "div-int", format = "23x", can_throw)]
    DivInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rem-int", format = "23x", can_throw)]
    RemInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "and-int", format = "23x")]
    AndInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "or-int", format = "23x")]
    OrInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "xor-int", format = "23x")]
    XorInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "shl-int", format = "23x")]
    ShlInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "shr-int", format = "23x")]
    ShrInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "ushr-int", format = "23x")]
    UshrInt(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "add-long", format = "23x")]
    AddLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "sub-long", format = "23x")]
    SubLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "mul-long", format = "23x")]
    MulLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "div-long", format = "23x", can_throw)]
    DivLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rem-long", format = "23x", can_throw)]
    RemLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "and-long", format = "23x")]
    AndLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "or-long", format = "23x")]
    OrLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "xor-long", format = "23x")]
    XorLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "shl-long", format = "23x")]
    ShlLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "shr-long", format = "23x")]
    ShrLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "ushr-long", format = "23x")]
    UshrLong(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "add-float", format = "23x")]
    AddFloat(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "sub-float", format = "23x")]
    SubFloat(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "mul-float", format = "23x")]
    MulFloat(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "div-float", format = "23x")]
    DivFloat(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rem-float", format = "23x")]
    RemFloat(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "add-double", format = "23x")]
    AddDouble(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "sub-double", format = "23x")]
    SubDouble(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "mul-double", format = "23x")]
    MulDouble(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "div-double", format = "23x")]
    DivDouble(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rem-double", format = "23x")]
    RemDouble(Reg, Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "add-int/2addr", format = "12x")]
    AddInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "sub-int/2addr", format = "12x")]
    SubInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "mul-int/2addr", format = "12x")]
    MulInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "div-int/2addr", format = "12x", can_throw)]
    DivInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "rem-int/2addr", format = "12x", can_throw)]
    RemInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "and-int/2addr", format = "12x")]
    AndInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "or-int/2addr", format = "12x")]
    OrInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "xor-int/2addr", format = "12x")]
    XorInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "shl-int/2addr", format = "12x")]
    ShlInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "shr-int/2addr", format = "12x")]
    ShrInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "ushr-int/2addr", format = "12x")]
    UshrInt2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "add-long/2addr", format = "12x")]
    AddLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "sub-long/2addr", format = "12x")]
    SubLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "mul-long/2addr", format = "12x")]
    MulLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "div-long/2addr", format = "12x", can_throw)]
    DivLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "rem-long/2addr", format = "12x", can_throw)]
    RemLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "and-long/2addr", format = "12x")]
    AndLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "or-long/2addr", format = "12x")]
    OrLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "xor-long/2addr", format = "12x")]
    XorLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "shl-long/2addr", format = "12x")]
    ShlLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "shr-long/2addr", format = "12x")]
    ShrLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "ushr-long/2addr", format = "12x")]
    UshrLong2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "add-float/2addr", format = "12x")]
    AddFloat2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "sub-float/2addr", format = "12x")]
    SubFloat2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "mul-float/2addr", format = "12x")]
    MulFloat2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "div-float/2addr", format = "12x")]
    DivFloat2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "rem-float/2addr", format = "12x")]
    RemFloat2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "add-double/2addr", format = "12x")]
    AddDouble2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "sub-double/2addr", format = "12x")]
    SubDouble2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "mul-double/2addr", format = "12x")]
    MulDouble2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "div-double/2addr", format = "12x")]
    DivDouble2addr(Reg, Reg),

    /// Perform the identified binary operation on the two source registers,
    /// storing the result in the first source register.
    #[instruction(mnemonic = "rem-double/2addr", format = "12x")]
    RemDouble2addr(Reg, Reg),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "add-int/lit16", format = "22s")]
    AddIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rsub-int", format = "22s")]
    RsubInt(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "mul-int/lit16", format = "22s")]
    MulIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "div-int/lit16", format = "22s", can_throw)]
    DivIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rem-int/lit16", format = "22s", can_throw)]
    RemIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "and-int/lit16", format = "22s")]
    AndIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "or-int/lit16", format = "22s")]
    OrIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "xor-int/lit16", format = "22s")]
    XorIntLit16(Reg, Reg, i16),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "add-int/lit8", format = "22b")]
    AddIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rsub-int/lit8", format = "22b")]
    RsubIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "mul-int/lit8", format = "22b")]
    MulIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "div-int/lit8", format = "22b", can_throw)]
    DivIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "rem-int/lit8", format = "22b", can_throw)]
    RemIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "and-int/lit8", format = "22b")]
    AndIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "or-int/lit8", format = "22b")]
    OrIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "xor-int/lit8", format = "22b")]
    XorIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "shl-int/lit8", format = "22b")]
    ShlIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "shr-int/lit8", format = "22b")]
    ShrIntLit8(Reg, Reg, i8),

    /// Perform the indicated binary op on the indicated register (first argument) and literal value (second argument),
    /// storing the result in the destination register.
    #[instruction(mnemonic = "ushr-int/lit8", format = "22b")]
    UshrIntLit8(Reg, Reg, i8),

    #[instruction(mnemonic = "invoke-polymorphic", format = "45cc", can_throw)]
    InvokePolymorphic(RegList, Index<MethodIdItem>, Index<ProtoIdItem>),
    #[instruction(mnemonic = "invoke-polymorphic/range", format = "4rcc", can_throw)]
    InvokePolymorphicRange(RegRange, Index<MethodIdItem>, Index<ProtoIdItem>),
    #[instruction(mnemonic = "invoke-custom", format = "35c", can_throw)]
    InvokeCustom(RegList, Index<CallSiteIdItem>),
    #[instruction(mnemonic = "invoke-custom/range", format = "3rc", can_throw)]
    InvokeCustomRange(RegRange, Index<CallSiteIdItem>),

    /// Move a reference to the method handle specified by the given index into the specified register.
    /// Present in Dex files from version 039 onwards.
    #[instruction(mnemonic = "const-method-handle", format = "21c")]
    ConstMethodHandle(Reg, Index<MethodHandleItem>),

    /// Move a reference to the method prototype specified by the given index into the specified register.
    /// Present in Dex files from version 039 onwards.
    #[instruction(mnemonic = "const-method-type", format = "21c")]
    ConstMethodType(Reg, Index<ProtoIdItem>),

    #[instruction(
        mnemonic = "packed-switch-payload",
        format = "custom",
        size = "(_1.len() * 2) + 4"
    )]
    PackedSwitchPayload(i32, Vec<i32>),

    #[instruction(
        mnemonic = "sparse-switch-payload",
        format = "custom",
        size = "(_1.len() * 4) + 2"
    )]
    SparseSwitchPayload(Vec<i32>, Vec<i32>),

    #[instruction(
        mnemonic = "fill-array-data-payload",
        format = "custom",
        size = "(_0.len() * _0.get(0).map_or(0, Vec::len) + 1) / 2 + 4"
    )]
    FillArrayDataPayload(Vec<Vec<u8>>),
}

impl PrettyPrint for Instr {
    #[allow(clippy::enum_glob_use)]
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        use Instr::*;
        write!(f, "{}", self.mnemonic())?;
        match self {
            Nop
            | PackedSwitchPayload(_, _)
            | SparseSwitchPayload(_, _)
            | FillArrayDataPayload(_)
            | ReturnVoid => Ok(()),

            Move(a, b)
            | MoveFrom16(a, b)
            | Move16(a, b)
            | MoveWide(a, b)
            | MoveWideFrom16(a, b)
            | MoveWide16(a, b)
            | MoveObject(a, b)
            | MoveObjectFrom16(a, b)
            | MoveObject16(a, b)
            | AddInt2addr(a, b)
            | SubInt2addr(a, b)
            | MulInt2addr(a, b)
            | DivInt2addr(a, b)
            | RemInt2addr(a, b)
            | AndInt2addr(a, b)
            | OrInt2addr(a, b)
            | XorInt2addr(a, b)
            | ShlInt2addr(a, b)
            | ShrInt2addr(a, b)
            | UshrInt2addr(a, b)
            | AddLong2addr(a, b)
            | SubLong2addr(a, b)
            | MulLong2addr(a, b)
            | DivLong2addr(a, b)
            | RemLong2addr(a, b)
            | AndLong2addr(a, b)
            | OrLong2addr(a, b)
            | XorLong2addr(a, b)
            | ShlLong2addr(a, b)
            | ShrLong2addr(a, b)
            | UshrLong2addr(a, b)
            | AddFloat2addr(a, b)
            | SubFloat2addr(a, b)
            | MulFloat2addr(a, b)
            | DivFloat2addr(a, b)
            | RemFloat2addr(a, b)
            | AddDouble2addr(a, b)
            | SubDouble2addr(a, b)
            | MulDouble2addr(a, b)
            | DivDouble2addr(a, b)
            | RemDouble2addr(a, b)
            | NegInt(a, b)
            | NotInt(a, b)
            | NegLong(a, b)
            | NotLong(a, b)
            | NegFloat(a, b)
            | NegDouble(a, b)
            | IntToLong(a, b)
            | IntToFloat(a, b)
            | IntToDouble(a, b)
            | LongToInt(a, b)
            | LongToFloat(a, b)
            | LongToDouble(a, b)
            | FloatToInt(a, b)
            | FloatToLong(a, b)
            | FloatToDouble(a, b)
            | DoubleToInt(a, b)
            | DoubleToLong(a, b)
            | DoubleToFloat(a, b)
            | IntToByte(a, b)
            | IntToChar(a, b)
            | IntToShort(a, b)
            | ArrayLength(a, b) => {
                write!(f, " {a}, {b}")?;
                Ok(())
            }

            MoveResult(a) | MoveResultWide(a) | MoveResultObject(a) | MoveException(a)
            | Return(a) | ReturnWide(a) | ReturnObject(a) | Throw(a) | MonitorEnter(a)
            | MonitorExit(a) => {
                write!(f, " {a}")?;
                Ok(())
            }

            Const4(a, b) => {
                write!(f, " {a}, #+{b:x}")?;
                Ok(())
            }
            Const16(a, b) | ConstWide16(a, b) => {
                write!(f, " {a}, #+{b:x}")?;
                Ok(())
            }
            Const(a, b) | ConstWide32(a, b) => {
                write!(f, " {a}, #+{b:x}")?;
                Ok(())
            }
            ConstHigh16(a, b) => {
                write!(f, " {a}, #+{b:x}0000")?;
                Ok(())
            }
            ConstWide(a, b) => {
                write!(f, " {a}, #+{b:x}")?;
                Ok(())
            }
            ConstWideHigh16(a, b) => {
                write!(f, " {a}, #+{b:x}000000000000")?;
                Ok(())
            }

            ConstString(a, s) | ConstStringJumbo(a, s) => {
                let s = s.get(dex)?.to_string(dex)?.replace('\n', "\\n");
                write!(f, " {a}, \"{s}\"")?;
                Ok(())
            }

            ConstClass(a, t) | CheckCast(a, t) | NewInstance(a, t) => {
                write!(f, " {a}, ")?;
                t.get(dex)?.pp(f, dex)
            }

            InstanceOf(a, b, t) | NewArray(a, b, t) => {
                write!(f, " {a}, {b}, ")?;
                t.get(dex)?.pp(f, dex)
            }

            FilledNewArray(args, t) => {
                write!(f, " {args}, ")?;
                t.get(dex)?.pp(f, dex)
            }
            FilledNewArrayRange(rr, t) => {
                write!(f, " {rr}, ")?;
                t.get(dex)?.pp(f, dex)
            }
            FillArrayData(a, b) | PackedSwitch(a, b) | SparseSwitch(a, b) => {
                write!(f, " {a}, +{b}")?;
                Ok(())
            }
            Goto(a) => {
                write!(f, " +{a}")?;
                Ok(())
            }
            Goto16(a) => {
                write!(f, " +{a}")?;
                Ok(())
            }
            Goto32(a) => {
                write!(f, " +{a}")?;
                Ok(())
            }

            CmplFloat(a, b, c)
            | CmpgFloat(a, b, c)
            | CmplDouble(a, b, c)
            | CmpgDouble(a, b, c)
            | CmpLong(a, b, c)
            | AddInt(a, b, c)
            | SubInt(a, b, c)
            | MulInt(a, b, c)
            | DivInt(a, b, c)
            | RemInt(a, b, c)
            | AndInt(a, b, c)
            | OrInt(a, b, c)
            | XorInt(a, b, c)
            | ShlInt(a, b, c)
            | ShrInt(a, b, c)
            | UshrInt(a, b, c)
            | AddLong(a, b, c)
            | SubLong(a, b, c)
            | MulLong(a, b, c)
            | DivLong(a, b, c)
            | RemLong(a, b, c)
            | AndLong(a, b, c)
            | OrLong(a, b, c)
            | XorLong(a, b, c)
            | ShlLong(a, b, c)
            | ShrLong(a, b, c)
            | UshrLong(a, b, c)
            | AddFloat(a, b, c)
            | SubFloat(a, b, c)
            | MulFloat(a, b, c)
            | DivFloat(a, b, c)
            | RemFloat(a, b, c)
            | AddDouble(a, b, c)
            | SubDouble(a, b, c)
            | MulDouble(a, b, c)
            | DivDouble(a, b, c)
            | RemDouble(a, b, c)
            | Aget(a, b, c)
            | AgetWide(a, b, c)
            | AgetObject(a, b, c)
            | AgetBoolean(a, b, c)
            | AgetByte(a, b, c)
            | AgetChar(a, b, c)
            | AgetShort(a, b, c)
            | Aput(a, b, c)
            | AputWide(a, b, c)
            | AputObject(a, b, c)
            | AputBoolean(a, b, c)
            | AputByte(a, b, c)
            | AputChar(a, b, c)
            | AputShort(a, b, c) => {
                write!(f, " {a}, {b}, {c}")?;
                Ok(())
            }

            IfEq(a, b, c)
            | IfNe(a, b, c)
            | IfLt(a, b, c)
            | IfGe(a, b, c)
            | IfGt(a, b, c)
            | IfLe(a, b, c) => {
                write!(f, " {a}, {b}, +{c}")?;
                Ok(())
            }

            IfEqz(a, b) | IfNez(a, b) | IfLtz(a, b) | IfGez(a, b) | IfGtz(a, b) | IfLez(a, b) => {
                write!(f, " {a}, +{b}")?;
                Ok(())
            }

            Iget(a, b, c)
            | IgetWide(a, b, c)
            | IgetObject(a, b, c)
            | IgetBoolean(a, b, c)
            | IgetByte(a, b, c)
            | IgetChar(a, b, c)
            | IgetShort(a, b, c)
            | Iput(a, b, c)
            | IputWide(a, b, c)
            | IputObject(a, b, c)
            | IputBoolean(a, b, c)
            | IputByte(a, b, c)
            | IputChar(a, b, c)
            | IputShort(a, b, c) => {
                write!(f, " {a}, {b}, ")?;
                c.get(dex)?.pp(f, dex)
            }

            Sget(a, b)
            | SgetWide(a, b)
            | SgetObject(a, b)
            | SgetBoolean(a, b)
            | SgetByte(a, b)
            | SgetChar(a, b)
            | SgetShort(a, b)
            | Sput(a, b)
            | SputWide(a, b)
            | SputObject(a, b)
            | SputBoolean(a, b)
            | SputByte(a, b)
            | SputChar(a, b)
            | SputShort(a, b) => {
                write!(f, " {a}, ")?;
                b.get(dex)?.pp(f, dex)
            }

            InvokeVirtual(args, b)
            | InvokeSuper(args, b)
            | InvokeDirect(args, b)
            | InvokeStatic(args, b)
            | InvokeInterface(args, b) => {
                write!(f, " {args}, ")?;
                b.get(dex)?.pp(f, dex)
            }

            InvokeVirtualRange(rr, c)
            | InvokeSuperRange(rr, c)
            | InvokeDirectRange(rr, c)
            | InvokeStaticRange(rr, c)
            | InvokeInterfaceRange(rr, c) => {
                write!(f, " {rr}, ")?;
                c.get(dex)?.pp(f, dex)
            }

            AddIntLit16(a, b, c)
            | RsubInt(a, b, c)
            | MulIntLit16(a, b, c)
            | DivIntLit16(a, b, c)
            | RemIntLit16(a, b, c)
            | AndIntLit16(a, b, c)
            | OrIntLit16(a, b, c)
            | XorIntLit16(a, b, c) => {
                write!(f, " {a}, {b}, #+{c}")?;
                Ok(())
            }

            AddIntLit8(a, b, c)
            | RsubIntLit8(a, b, c)
            | MulIntLit8(a, b, c)
            | DivIntLit8(a, b, c)
            | RemIntLit8(a, b, c)
            | AndIntLit8(a, b, c)
            | OrIntLit8(a, b, c)
            | XorIntLit8(a, b, c)
            | ShlIntLit8(a, b, c)
            | ShrIntLit8(a, b, c)
            | UshrIntLit8(a, b, c) => {
                write!(f, " {a}, {b}, #+{c}")?;
                Ok(())
            }

            InvokePolymorphic(args, b, c) => {
                write!(f, " {args}, ")?;
                b.get(dex)?.pp(f, dex)?;
                write!(f, ", ")?;
                c.get(dex)?.pp(f, dex)
            }
            InvokePolymorphicRange(rr, meth, proto) => {
                write!(f, " {rr}, ")?;
                meth.get(dex)?.pp(f, dex)?;
                write!(f, ", ")?;
                proto.get(dex)?.pp(f, dex)
            }
            InvokeCustom(args, callsite) => {
                write!(f, " {args}, ")?;
                callsite.get(dex)?.pp(f, dex)
            }
            InvokeCustomRange(rr, callsite) => {
                write!(f, " {rr}, ")?;
                callsite.get(dex)?.pp(f, dex)
            }
            ConstMethodHandle(a, handle) => {
                write!(f, " {a}, ")?;
                handle.get(dex)?.pp(f, dex)
            }
            ConstMethodType(a, proto) => {
                write!(f, " {a}, ")?;
                proto.get(dex)?.pp(f, dex)
            }
        }
    }
}

impl<'a> Serialize for WithDex<'a, Instr> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let operands = collect_operands::<S>(&self.data, self.dex)?;

        let mut state = serializer.serialize_struct("Instr", 2)?;
        state.serialize_field("mnemonic", &self.data.mnemonic())?;
        state.serialize_field("operands", &operands)?;
        state.end()
    }
}

// Operands for serialization:

#[derive(Debug, Serialize)]
enum Operand {
    Reg(Reg),
    RegList(RegList),
    RegRange(RegRange),
    Const(i64),
    Addr(i32),
    String(String),
    Type(Type),
    Proto {
        return_: Type,
        params: Vec<Type>,
    },
    Field {
        classname: String,
        name: String,
        type_: Type,
    },
    Method {
        definer: Type,
        name: String,
        return_: Type,
        params: Vec<Type>,
    },
    CallSite(Vec<OpValue>),
    MethodHandle {
        handle_kind: OpMethodHandle,
        handle_arg: Box<Operand>,
    },
}

#[derive(Debug, Serialize)]
enum OpValue {
    Byte(i8),
    Short(i16),
    Char(u16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    MethodType {
        return_: Type,
        params: Vec<Type>,
    },
    MethodHandle {
        handle_kind: OpMethodHandle,
        handle_arg: Box<Operand>,
    },
    String(String),
    Type(Type),
    Field {
        classname: String,
        name: String,
        type_: Type,
    },
    Method {
        definer: Type,
        name: String,
        return_: Type,
        params: Vec<Type>,
    },
    Enum {
        classname: String,
        name: String,
        type_: Type,
    },
    Array(Vec<OpValue>),
    Annotation {
        type_: Type,
        elements: Vec<(String, OpValue)>,
    },
    Null,
    Boolean(bool),
}

#[derive(Debug, Serialize)]
enum OpMethodHandle {
    StaticPut,
    StaticGet,
    InstancePut,
    InstanceGet,
    InvokeStatic,
    InvokeInstance,
    InvokeConstructor,
    InvokeDirect,
    InvokeInterface,
}

fn collect_operands<S: Serializer>(instr: &Instr, dex: &Dex) -> Result<Vec<Operand>, S::Error> {
    match instr {
        Instr::Nop
        | Instr::PackedSwitchPayload(_, _)
        | Instr::SparseSwitchPayload(_, _)
        | Instr::FillArrayDataPayload(_)
        | Instr::ReturnVoid => Ok(vec![]),

        Instr::Move(a, b)
        | Instr::MoveFrom16(a, b)
        | Instr::Move16(a, b)
        | Instr::MoveWide(a, b)
        | Instr::MoveWideFrom16(a, b)
        | Instr::MoveWide16(a, b)
        | Instr::MoveObject(a, b)
        | Instr::MoveObjectFrom16(a, b)
        | Instr::MoveObject16(a, b)
        | Instr::AddInt2addr(a, b)
        | Instr::SubInt2addr(a, b)
        | Instr::MulInt2addr(a, b)
        | Instr::DivInt2addr(a, b)
        | Instr::RemInt2addr(a, b)
        | Instr::AndInt2addr(a, b)
        | Instr::OrInt2addr(a, b)
        | Instr::XorInt2addr(a, b)
        | Instr::ShlInt2addr(a, b)
        | Instr::ShrInt2addr(a, b)
        | Instr::UshrInt2addr(a, b)
        | Instr::AddLong2addr(a, b)
        | Instr::SubLong2addr(a, b)
        | Instr::MulLong2addr(a, b)
        | Instr::DivLong2addr(a, b)
        | Instr::RemLong2addr(a, b)
        | Instr::AndLong2addr(a, b)
        | Instr::OrLong2addr(a, b)
        | Instr::XorLong2addr(a, b)
        | Instr::ShlLong2addr(a, b)
        | Instr::ShrLong2addr(a, b)
        | Instr::UshrLong2addr(a, b)
        | Instr::AddFloat2addr(a, b)
        | Instr::SubFloat2addr(a, b)
        | Instr::MulFloat2addr(a, b)
        | Instr::DivFloat2addr(a, b)
        | Instr::RemFloat2addr(a, b)
        | Instr::AddDouble2addr(a, b)
        | Instr::SubDouble2addr(a, b)
        | Instr::MulDouble2addr(a, b)
        | Instr::DivDouble2addr(a, b)
        | Instr::RemDouble2addr(a, b)
        | Instr::NegInt(a, b)
        | Instr::NotInt(a, b)
        | Instr::NegLong(a, b)
        | Instr::NotLong(a, b)
        | Instr::NegFloat(a, b)
        | Instr::NegDouble(a, b)
        | Instr::IntToLong(a, b)
        | Instr::IntToFloat(a, b)
        | Instr::IntToDouble(a, b)
        | Instr::LongToInt(a, b)
        | Instr::LongToFloat(a, b)
        | Instr::LongToDouble(a, b)
        | Instr::FloatToInt(a, b)
        | Instr::FloatToLong(a, b)
        | Instr::FloatToDouble(a, b)
        | Instr::DoubleToInt(a, b)
        | Instr::DoubleToLong(a, b)
        | Instr::DoubleToFloat(a, b)
        | Instr::IntToByte(a, b)
        | Instr::IntToChar(a, b)
        | Instr::IntToShort(a, b)
        | Instr::ArrayLength(a, b) => Ok(vec![Operand::Reg(*a), Operand::Reg(*b)]),

        Instr::MoveResult(a)
        | Instr::MoveResultWide(a)
        | Instr::MoveResultObject(a)
        | Instr::MoveException(a)
        | Instr::Return(a)
        | Instr::ReturnWide(a)
        | Instr::ReturnObject(a)
        | Instr::Throw(a)
        | Instr::MonitorEnter(a)
        | Instr::MonitorExit(a) => Ok(vec![Operand::Reg(*a)]),

        Instr::Const4(a, b) => Ok(vec![Operand::Reg(*a), Operand::Const(i64::from(*b))]),
        Instr::Const16(a, b) | Instr::ConstWide16(a, b) => {
            Ok(vec![Operand::Reg(*a), Operand::Const(i64::from(*b))])
        }
        Instr::Const(a, b) | Instr::ConstWide32(a, b) => {
            Ok(vec![Operand::Reg(*a), Operand::Const(i64::from(*b))])
        }
        Instr::ConstHigh16(a, b) => Ok(vec![Operand::Reg(*a), Operand::Const(i64::from(*b) << 16)]),
        Instr::ConstWide(a, b) => Ok(vec![Operand::Reg(*a), Operand::Const(*b)]),
        Instr::ConstWideHigh16(a, b) => {
            Ok(vec![Operand::Reg(*a), Operand::Const(i64::from(*b) << 48)])
        }
        Instr::ConstString(a, s) | Instr::ConstStringJumbo(a, s) => {
            Ok(vec![Operand::Reg(*a), operand_string::<S>(*s, dex)?])
        }
        Instr::ConstClass(a, t) => Ok(vec![Operand::Reg(*a), operand_type::<S>(*t, dex)?]),
        Instr::CheckCast(a, t) => Ok(vec![Operand::Reg(*a), operand_type::<S>(*t, dex)?]),
        Instr::InstanceOf(a, b, t) => Ok(vec![
            Operand::Reg(*a),
            Operand::Reg(*b),
            operand_type::<S>(*t, dex)?,
        ]),
        Instr::NewInstance(a, t) => Ok(vec![Operand::Reg(*a), operand_type::<S>(*t, dex)?]),
        Instr::NewArray(a, b, t) => Ok(vec![
            Operand::Reg(*a),
            Operand::Reg(*b),
            operand_type::<S>(*t, dex)?,
        ]),
        Instr::FilledNewArray(args, t) => Ok(vec![
            Operand::RegList(args.clone()),
            operand_type::<S>(*t, dex)?,
        ]),
        Instr::FilledNewArrayRange(rr, t) => {
            Ok(vec![Operand::RegRange(*rr), operand_type::<S>(*t, dex)?])
        }
        Instr::FillArrayData(a, b) => Ok(vec![Operand::Reg(*a), Operand::Addr(*b)]),
        Instr::Goto(a) => Ok(vec![Operand::Addr(i32::from(*a))]),
        Instr::Goto16(a) => Ok(vec![Operand::Addr(i32::from(*a))]),
        Instr::Goto32(a) => Ok(vec![Operand::Addr(*a)]),
        Instr::PackedSwitch(a, b) | Instr::SparseSwitch(a, b) => {
            Ok(vec![Operand::Reg(*a), Operand::Addr(*b)])
        }

        Instr::CmplFloat(a, b, c)
        | Instr::CmpgFloat(a, b, c)
        | Instr::CmplDouble(a, b, c)
        | Instr::CmpgDouble(a, b, c)
        | Instr::CmpLong(a, b, c)
        | Instr::AddInt(a, b, c)
        | Instr::SubInt(a, b, c)
        | Instr::MulInt(a, b, c)
        | Instr::DivInt(a, b, c)
        | Instr::RemInt(a, b, c)
        | Instr::AndInt(a, b, c)
        | Instr::OrInt(a, b, c)
        | Instr::XorInt(a, b, c)
        | Instr::ShlInt(a, b, c)
        | Instr::ShrInt(a, b, c)
        | Instr::UshrInt(a, b, c)
        | Instr::AddLong(a, b, c)
        | Instr::SubLong(a, b, c)
        | Instr::MulLong(a, b, c)
        | Instr::DivLong(a, b, c)
        | Instr::RemLong(a, b, c)
        | Instr::AndLong(a, b, c)
        | Instr::OrLong(a, b, c)
        | Instr::XorLong(a, b, c)
        | Instr::ShlLong(a, b, c)
        | Instr::ShrLong(a, b, c)
        | Instr::UshrLong(a, b, c)
        | Instr::AddFloat(a, b, c)
        | Instr::SubFloat(a, b, c)
        | Instr::MulFloat(a, b, c)
        | Instr::DivFloat(a, b, c)
        | Instr::RemFloat(a, b, c)
        | Instr::AddDouble(a, b, c)
        | Instr::SubDouble(a, b, c)
        | Instr::MulDouble(a, b, c)
        | Instr::DivDouble(a, b, c)
        | Instr::RemDouble(a, b, c)
        | Instr::Aget(a, b, c)
        | Instr::AgetWide(a, b, c)
        | Instr::AgetObject(a, b, c)
        | Instr::AgetBoolean(a, b, c)
        | Instr::AgetByte(a, b, c)
        | Instr::AgetChar(a, b, c)
        | Instr::AgetShort(a, b, c)
        | Instr::Aput(a, b, c)
        | Instr::AputWide(a, b, c)
        | Instr::AputObject(a, b, c)
        | Instr::AputBoolean(a, b, c)
        | Instr::AputByte(a, b, c)
        | Instr::AputChar(a, b, c)
        | Instr::AputShort(a, b, c) => {
            Ok(vec![Operand::Reg(*a), Operand::Reg(*b), Operand::Reg(*c)])
        }

        Instr::IfEq(a, b, c)
        | Instr::IfNe(a, b, c)
        | Instr::IfLt(a, b, c)
        | Instr::IfGe(a, b, c)
        | Instr::IfGt(a, b, c)
        | Instr::IfLe(a, b, c) => Ok(vec![
            Operand::Reg(*a),
            Operand::Reg(*b),
            Operand::Addr(i32::from(*c)),
        ]),
        Instr::IfEqz(a, b)
        | Instr::IfNez(a, b)
        | Instr::IfLtz(a, b)
        | Instr::IfGez(a, b)
        | Instr::IfGtz(a, b)
        | Instr::IfLez(a, b) => Ok(vec![Operand::Reg(*a), Operand::Addr(i32::from(*b))]),

        Instr::Iget(a, b, c)
        | Instr::IgetWide(a, b, c)
        | Instr::IgetObject(a, b, c)
        | Instr::IgetBoolean(a, b, c)
        | Instr::IgetByte(a, b, c)
        | Instr::IgetChar(a, b, c)
        | Instr::IgetShort(a, b, c)
        | Instr::Iput(a, b, c)
        | Instr::IputWide(a, b, c)
        | Instr::IputObject(a, b, c)
        | Instr::IputBoolean(a, b, c)
        | Instr::IputByte(a, b, c)
        | Instr::IputChar(a, b, c)
        | Instr::IputShort(a, b, c) => Ok(vec![
            Operand::Reg(*a),
            Operand::Reg(*b),
            operand_field::<S>(*c, dex)?,
        ]),
        Instr::Sget(a, b)
        | Instr::SgetWide(a, b)
        | Instr::SgetObject(a, b)
        | Instr::SgetBoolean(a, b)
        | Instr::SgetByte(a, b)
        | Instr::SgetChar(a, b)
        | Instr::SgetShort(a, b)
        | Instr::Sput(a, b)
        | Instr::SputWide(a, b)
        | Instr::SputObject(a, b)
        | Instr::SputBoolean(a, b)
        | Instr::SputByte(a, b)
        | Instr::SputChar(a, b)
        | Instr::SputShort(a, b) => Ok(vec![Operand::Reg(*a), operand_field::<S>(*b, dex)?]),
        Instr::InvokeVirtual(args, b)
        | Instr::InvokeSuper(args, b)
        | Instr::InvokeDirect(args, b)
        | Instr::InvokeStatic(args, b)
        | Instr::InvokeInterface(args, b) => Ok(vec![
            Operand::RegList(args.clone()),
            operand_method::<S>(*b, dex)?,
        ]),
        Instr::InvokeVirtualRange(rr, c)
        | Instr::InvokeSuperRange(rr, c)
        | Instr::InvokeDirectRange(rr, c)
        | Instr::InvokeStaticRange(rr, c)
        | Instr::InvokeInterfaceRange(rr, c) => {
            Ok(vec![Operand::RegRange(*rr), operand_method::<S>(*c, dex)?])
        }

        Instr::AddIntLit16(a, b, c)
        | Instr::RsubInt(a, b, c)
        | Instr::MulIntLit16(a, b, c)
        | Instr::DivIntLit16(a, b, c)
        | Instr::RemIntLit16(a, b, c)
        | Instr::AndIntLit16(a, b, c)
        | Instr::OrIntLit16(a, b, c)
        | Instr::XorIntLit16(a, b, c) => Ok(vec![
            Operand::Reg(*a),
            Operand::Reg(*b),
            Operand::Const(i64::from(*c)),
        ]),

        Instr::AddIntLit8(a, b, c)
        | Instr::RsubIntLit8(a, b, c)
        | Instr::MulIntLit8(a, b, c)
        | Instr::DivIntLit8(a, b, c)
        | Instr::RemIntLit8(a, b, c)
        | Instr::AndIntLit8(a, b, c)
        | Instr::OrIntLit8(a, b, c)
        | Instr::XorIntLit8(a, b, c)
        | Instr::ShlIntLit8(a, b, c)
        | Instr::ShrIntLit8(a, b, c)
        | Instr::UshrIntLit8(a, b, c) => Ok(vec![
            Operand::Reg(*a),
            Operand::Reg(*b),
            Operand::Const(i64::from(*c)),
        ]),

        Instr::InvokePolymorphic(args, meth, proto) => Ok(vec![
            Operand::RegList(args.clone()),
            operand_method::<S>(*meth, dex)?,
            operand_proto::<S>(*proto, dex)?,
        ]),
        Instr::InvokePolymorphicRange(rr, meth, proto) => Ok(vec![
            Operand::RegRange(*rr),
            operand_method::<S>(*meth, dex)?,
            operand_proto::<S>(*proto, dex)?,
        ]),

        Instr::InvokeCustom(args, cs) => Ok(vec![
            Operand::RegList(args.clone()),
            operand_call_site::<S>(*cs, dex)?,
        ]),
        Instr::InvokeCustomRange(rr, cs) => Ok(vec![
            Operand::RegRange(*rr),
            operand_call_site::<S>(*cs, dex)?,
        ]),

        Instr::ConstMethodHandle(a, h) => {
            Ok(vec![Operand::Reg(*a), operand_method_handle::<S>(*h, dex)?])
        }
        Instr::ConstMethodType(a, proto) => {
            Ok(vec![Operand::Reg(*a), operand_proto::<S>(*proto, dex)?])
        }
    }
}

fn operand_string<S: Serializer>(s: Index<StringIdItem>, dex: &Dex) -> Result<Operand, S::Error> {
    let s = s
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?
        .to_string(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?
        .replace('\n', "\\n");
    Ok(Operand::String(s))
}

fn operand_type<S: Serializer>(t: Index<TypeIdItem>, dex: &Dex) -> Result<Operand, S::Error> {
    let typ = t
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?
        .to_type(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    Ok(Operand::Type(typ))
}

fn operand_proto<S: Serializer>(p: Index<ProtoIdItem>, dex: &Dex) -> Result<Operand, S::Error> {
    let proto = p
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let proto_return_ = proto
        .return_type(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let proto_params = proto
        .parameters_types(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    Ok(Operand::Proto {
        return_: proto_return_,
        params: proto_params,
    })
}

fn operand_field<S: Serializer>(f: Index<FieldIdItem>, dex: &Dex) -> Result<Operand, S::Error> {
    let field = f
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let classname = field
        .class_name(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let name = field
        .name(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let type_ = field
        .type_(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    Ok(Operand::Field {
        classname,
        name,
        type_,
    })
}

fn operand_method<S: Serializer>(m: Index<MethodIdItem>, dex: &Dex) -> Result<Operand, S::Error> {
    let method = m
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let definer = method
        .definer(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let name = method
        .name(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let return_ = method
        .return_type(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let params = method
        .parameters_types(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    Ok(Operand::Method {
        definer,
        name,
        return_,
        params,
    })
}

fn operand_call_site<S: Serializer>(
    cs: Index<CallSiteIdItem>,
    dex: &Dex,
) -> Result<Operand, S::Error> {
    let cs_args = cs
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?
        .arguments(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?
        .values
        .iter()
        .map(|val| op_value::<S>(val, dex))
        .collect::<Result<Vec<OpValue>, S::Error>>()?;
    Ok(Operand::CallSite(cs_args))
}

fn operand_method_handle<S: Serializer>(
    h: Index<MethodHandleItem>,
    dex: &Dex,
) -> Result<Operand, S::Error> {
    let handle = h
        .get(dex)
        .map_err(|err| ser::Error::custom(format!("{err}")))?;
    let handle_kind = match handle.method_handle {
        MethodHandle::StaticPut(_) => OpMethodHandle::StaticPut,
        MethodHandle::StaticGet(_) => OpMethodHandle::StaticGet,
        MethodHandle::InstancePut(_) => OpMethodHandle::InstancePut,
        MethodHandle::InstanceGet(_) => OpMethodHandle::InstanceGet,
        MethodHandle::InvokeStatic(_) => OpMethodHandle::InvokeStatic,
        MethodHandle::InvokeInstance(_) => OpMethodHandle::InvokeInstance,
        MethodHandle::InvokeConstructor(_) => OpMethodHandle::InvokeConstructor,
        MethodHandle::InvokeDirect(_) => OpMethodHandle::InvokeDirect,
        MethodHandle::InvokeInterface(_) => OpMethodHandle::InvokeInterface,
    };
    let handle_arg = match handle.method_handle {
        MethodHandle::StaticPut(field)
        | MethodHandle::StaticGet(field)
        | MethodHandle::InstancePut(field)
        | MethodHandle::InstanceGet(field) => {
            let field = field
                .get(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let classname = field
                .class_name(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let name = field
                .name(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let type_ = field
                .type_(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            Operand::Field {
                classname,
                name,
                type_,
            }
        }
        MethodHandle::InvokeStatic(meth)
        | MethodHandle::InvokeInstance(meth)
        | MethodHandle::InvokeConstructor(meth)
        | MethodHandle::InvokeDirect(meth)
        | MethodHandle::InvokeInterface(meth) => {
            let meth = meth
                .get(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let definer = meth
                .definer(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let name = meth
                .name(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let return_ = meth
                .return_type(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            let params = meth
                .parameters_types(dex)
                .map_err(|err| ser::Error::custom(format!("{err}")))?;
            Operand::Method {
                definer,
                name,
                return_,
                params,
            }
        }
    };
    Ok(Operand::MethodHandle {
        handle_kind,
        handle_arg: Box::new(handle_arg),
    })
}

fn op_value<S: Serializer>(value: &EncodedValue, dex: &Dex) -> Result<OpValue, S::Error> {
    match value {
        EncodedValue::Byte(v) => Ok(OpValue::Byte(*v)),
        EncodedValue::Short(_, v) => Ok(OpValue::Short(*v)),
        EncodedValue::Char(_, v) => Ok(OpValue::Char(*v)),
        EncodedValue::Int(_, v) => Ok(OpValue::Int(*v)),
        EncodedValue::Long(_, v) => Ok(OpValue::Long(*v)),
        EncodedValue::Float(_, v) => Ok(OpValue::Float(*v)),
        EncodedValue::Double(_, v) => Ok(OpValue::Double(*v)),
        EncodedValue::MethodType(_, proto) => {
            let Operand::Proto { return_, params } = operand_proto::<S>(*proto, dex)? else { unreachable!() };
            Ok(OpValue::MethodType { return_, params })
        }
        EncodedValue::MethodHandle(_, handle) => {
            let Operand::MethodHandle {
                    handle_kind,
                    handle_arg,
                } = operand_method_handle::<S>(*handle, dex)? else { unreachable!() };
            Ok(OpValue::MethodHandle {
                handle_kind,
                handle_arg,
            })
        }
        EncodedValue::String(_, s) => {
            let Operand::String(string) = operand_string::<S>(*s, dex)? else { unreachable!() };
            Ok(OpValue::String(string))
        }
        EncodedValue::Type(_, typ) => {
            let Operand::Type(t) = operand_type::<S>(*typ, dex)? else { unreachable!() };
            Ok(OpValue::Type(t))
        }
        EncodedValue::Field(_, field) => {
            let Operand::Field {
                    classname,
                    name,
                    type_,
                } = operand_field::<S>(*field, dex)? else { unreachable!() };
            Ok(OpValue::Field {
                classname,
                name,
                type_,
            })
        }
        EncodedValue::Method(_, method) => {
            let Operand::Method {
                    definer,
                    name,
                    return_,
                    params,
                } = operand_method::<S>(*method, dex)? else { unreachable!() };
            Ok(OpValue::Method {
                definer,
                name,
                return_,
                params,
            })
        }
        EncodedValue::Enum(_, field) => {
            let Operand::Field {
                    classname,
                    name,
                    type_,
                } = operand_field::<S>(*field, dex)? else { unreachable!() };
            Ok(OpValue::Enum {
                classname,
                name,
                type_,
            })
        }
        EncodedValue::Array(arr) => {
            let vals = arr
                .values
                .iter()
                .map(|val| op_value::<S>(val, dex))
                .collect::<Result<Vec<OpValue>, S::Error>>()?;
            Ok(OpValue::Array(vals))
        }
        EncodedValue::Annotation(ann) => {
            let Operand::Type(type_) = operand_type::<S>(ann.type_idx, dex)? else { unreachable!() };
            let elements = ann.elements.iter().map(|elt| {
                let Operand::String(s) = operand_string::<S>(elt.name_idx, dex)? else { unreachable!() };
                let v = op_value::<S>(&elt.value, dex)?;
                Ok((s, v))
            }).collect::<Result<Vec<(String, OpValue)>, S::Error>>()?;
            Ok(OpValue::Annotation { type_, elements })
        }
        EncodedValue::Null => Ok(OpValue::Null),
        EncodedValue::Boolean(b) => Ok(OpValue::Boolean(*b)),
    }
}
