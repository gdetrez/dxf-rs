use std::io::{Read, Write};

use crate::{CodePair, DxfError, DxfResult, Point, Vector};

use crate::code_pair_put_back::CodePairPutBack;
use crate::code_pair_writer::CodePairWriter;
use crate::enums::AcadVersion;
use crate::helper_functions::*;

pub(crate) const XDATA_APPLICATIONNAME: i32 = 1001;
const XDATA_STRING: i32 = 1000;
const XDATA_CONTROLGROUP: i32 = 1002;
const XDATA_LAYER: i32 = 1003;
const XDATA_BINARYDATA: i32 = 1004;
const XDATA_HANDLE: i32 = 1005;
const XDATA_THREEREALS: i32 = 1010;
const XDATA_WORLDSPACEPOSITION: i32 = 1011;
const XDATA_WORLDSPACEDISPLACEMENT: i32 = 1012;
const XDATA_WORLDDIRECTION: i32 = 1013;
const XDATA_REAL: i32 = 1040;
const XDATA_DISTANCE: i32 = 1041;
const XDATA_SCALEFACTOR: i32 = 1042;
const XDATA_INTEGER: i32 = 1070;
const XDATA_LONG: i32 = 1071;

/// Represents an application name and a collection of extended data.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct XData {
    pub application_name: String,
    pub items: Vec<XDataItem>,
}

/// Represents a piece of extended data.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum XDataItem {
    Str(String),
    ControlGroup(Vec<XDataItem>),
    LayerName(String),
    BinaryData(Vec<u8>),
    Handle(u32),
    ThreeReals(f64, f64, f64),
    WorldSpacePosition(Point),
    WorldSpaceDisplacement(Point),
    WorldDirection(Vector),
    Real(f64),
    Distance(f64),
    ScaleFactor(f64),
    Integer(i16),
    Long(i32),
}

impl XData {
    pub(crate) fn read_item<I>(
        application_name: String,
        iter: &mut CodePairPutBack<I>,
    ) -> DxfResult<XData>
    where
        I: Read,
    {
        let mut xdata = XData {
            application_name,
            items: vec![],
        };
        loop {
            let pair = match iter.next() {
                Some(Ok(pair @ CodePair { code: 0, .. })) => {
                    iter.put_back(Ok(pair));
                    return Ok(xdata);
                }
                Some(Ok(pair)) => pair,
                Some(Err(e)) => return Err(e),
                None => return Ok(xdata),
            };
            if pair.code == XDATA_APPLICATIONNAME || pair.code < XDATA_STRING {
                // new xdata or non xdata
                break;
            }
            xdata.items.push(XDataItem::read_item(&pair, iter)?);
        }
        Ok(xdata)
    }
    pub(crate) fn write<T>(
        &self,
        version: AcadVersion,
        writer: &mut CodePairWriter<T>,
    ) -> DxfResult<()>
    where
        T: Write + ?Sized,
    {
        // not supported on < R2000
        if version >= AcadVersion::R2000 {
            writer.write_code_pair(&CodePair::new_string(
                XDATA_APPLICATIONNAME,
                &self.application_name,
            ))?;
            for item in &self.items {
                item.write(writer)?;
            }
        }
        Ok(())
    }
}

impl XDataItem {
    fn read_item<I>(pair: &CodePair, iter: &mut CodePairPutBack<I>) -> DxfResult<XDataItem>
    where
        I: Read,
    {
        match pair.code {
            XDATA_STRING => Ok(XDataItem::Str(pair.assert_string()?)),
            XDATA_CONTROLGROUP => {
                let mut items = vec![];
                loop {
                    let pair = match iter.next() {
                        Some(Ok(pair)) => {
                            if pair.code < XDATA_STRING {
                                return Err(DxfError::UnexpectedCodePair(
                                    pair,
                                    String::from("expected XDATA item"),
                                ));
                            }
                            pair
                        }
                        Some(Err(e)) => return Err(e),
                        None => return Err(DxfError::UnexpectedEndOfInput),
                    };
                    if pair.code == XDATA_CONTROLGROUP && pair.assert_string()? == "}" {
                        // end of group
                        break;
                    }

                    items.push(XDataItem::read_item(&pair, iter)?);
                }
                Ok(XDataItem::ControlGroup(items))
            }
            XDATA_LAYER => Ok(XDataItem::LayerName(pair.assert_string()?)),
            XDATA_BINARYDATA => {
                let mut data = vec![];
                parse_hex_string(&pair.assert_string()?, &mut data, pair.offset)?;
                Ok(XDataItem::BinaryData(data))
            }
            XDATA_HANDLE => Ok(XDataItem::Handle(pair.as_handle()?)),
            XDATA_THREEREALS => Ok(XDataItem::ThreeReals(
                pair.assert_f64()?,
                XDataItem::read_double(iter, pair.code)?,
                XDataItem::read_double(iter, pair.code)?,
            )),
            XDATA_WORLDSPACEDISPLACEMENT => Ok(XDataItem::WorldSpaceDisplacement(
                XDataItem::read_point(iter, pair.assert_f64()?, pair.code)?,
            )),
            XDATA_WORLDSPACEPOSITION => Ok(XDataItem::WorldSpacePosition(XDataItem::read_point(
                iter,
                pair.assert_f64()?,
                pair.code,
            )?)),
            XDATA_WORLDDIRECTION => Ok(XDataItem::WorldDirection(XDataItem::read_vector(
                iter,
                pair.assert_f64()?,
                pair.code,
            )?)),
            XDATA_REAL => Ok(XDataItem::Real(pair.assert_f64()?)),
            XDATA_DISTANCE => Ok(XDataItem::Distance(pair.assert_f64()?)),
            XDATA_SCALEFACTOR => Ok(XDataItem::ScaleFactor(pair.assert_f64()?)),
            XDATA_INTEGER => Ok(XDataItem::Integer(pair.assert_i16()?)),
            XDATA_LONG => Ok(XDataItem::Long(pair.assert_i32()?)),
            _ => Err(DxfError::UnexpectedCode(pair.code, pair.offset)),
        }
    }
    fn read_double<T>(iter: &mut CodePairPutBack<T>, expected_code: i32) -> DxfResult<f64>
    where
        T: Read,
    {
        match iter.next() {
            Some(Ok(ref pair)) if pair.code == expected_code => Ok(pair.assert_f64()?),
            Some(Ok(pair)) => Err(DxfError::UnexpectedCode(pair.code, pair.offset)),
            Some(Err(e)) => Err(e),
            None => Err(DxfError::UnexpectedEndOfInput),
        }
    }
    fn read_point<I>(
        iter: &mut CodePairPutBack<I>,
        first: f64,
        expected_code: i32,
    ) -> DxfResult<Point>
    where
        I: Read,
    {
        Ok(Point::new(
            first,
            XDataItem::read_double(iter, expected_code)?,
            XDataItem::read_double(iter, expected_code)?,
        ))
    }
    fn read_vector<I>(
        iter: &mut CodePairPutBack<I>,
        first: f64,
        expected_code: i32,
    ) -> DxfResult<Vector>
    where
        I: Read,
    {
        Ok(Vector::new(
            first,
            XDataItem::read_double(iter, expected_code)?,
            XDataItem::read_double(iter, expected_code)?,
        ))
    }
    pub(crate) fn write<T>(&self, writer: &mut CodePairWriter<T>) -> DxfResult<()>
    where
        T: Write + ?Sized,
    {
        match self {
            XDataItem::Str(ref s) => {
                writer.write_code_pair(&CodePair::new_string(XDATA_STRING, s))?;
            }
            XDataItem::ControlGroup(ref items) => {
                writer.write_code_pair(&CodePair::new_str(XDATA_CONTROLGROUP, "{"))?;
                for item in &items[..] {
                    item.write(writer)?;
                }
                writer.write_code_pair(&CodePair::new_str(XDATA_CONTROLGROUP, "}"))?;
            }
            XDataItem::LayerName(ref l) => {
                writer.write_code_pair(&CodePair::new_string(XDATA_LAYER, l))?;
            }
            XDataItem::BinaryData(ref data) => {
                let mut line = String::new();
                for b in data {
                    line.push_str(&format!("{:02X}", b));
                }
                writer.write_code_pair(&CodePair::new_string(XDATA_BINARYDATA, &line))?;
            }
            XDataItem::Handle(h) => {
                writer.write_code_pair(&CodePair::new_string(XDATA_HANDLE, &as_handle(*h)))?;
            }
            XDataItem::ThreeReals(x, y, z) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_THREEREALS, *x))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_THREEREALS, *y))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_THREEREALS, *z))?;
            }
            XDataItem::WorldSpacePosition(ref p) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDSPACEPOSITION, p.x))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDSPACEPOSITION, p.y))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDSPACEPOSITION, p.z))?;
            }
            XDataItem::WorldSpaceDisplacement(ref p) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDSPACEDISPLACEMENT, p.x))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDSPACEDISPLACEMENT, p.y))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDSPACEDISPLACEMENT, p.z))?;
            }
            XDataItem::WorldDirection(ref v) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDDIRECTION, v.x))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDDIRECTION, v.y))?;
                writer.write_code_pair(&CodePair::new_f64(XDATA_WORLDDIRECTION, v.z))?;
            }
            XDataItem::Real(f) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_REAL, *f))?;
            }
            XDataItem::Distance(f) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_DISTANCE, *f))?;
            }
            XDataItem::ScaleFactor(f) => {
                writer.write_code_pair(&CodePair::new_f64(XDATA_SCALEFACTOR, *f))?;
            }
            XDataItem::Integer(i) => {
                writer.write_code_pair(&CodePair::new_i16(XDATA_INTEGER, *i))?;
            }
            XDataItem::Long(i) => {
                writer.write_code_pair(&CodePair::new_i32(XDATA_LONG, *i))?;
            }
        }
        Ok(())
    }
}
