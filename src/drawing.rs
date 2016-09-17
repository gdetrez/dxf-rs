// Copyright (c) IxMilia.  All Rights Reserved.  Licensed under the Apache License, Version 2.0.  See License.txt in the project root for license information.

use entities::*;
use enums::*;
use header::*;
use tables::*;

use ::{
    CodePair,
    CodePairAsciiIter,
    CodePairAsciiWriter,
    CodePairValue,
    DxfError,
    DxfResult,
    EntityIter,
};

use std::fs::File;
use std::io::{
    BufReader,
    BufWriter,
    Read,
    Write,
};
use std::path::Path;
use itertools::PutBack;

/// Represents a DXF drawing.
pub struct Drawing {
    /// The drawing's header.  Contains various drawing-specific values and settings.
    pub header: Header,
    /// The entities contained by the drawing.
    pub entities: Vec<Entity>,
    /// The AppIds contained by the drawing.
    pub app_ids: Vec<AppId>,
    /// The block records contained by the drawing.
    pub block_records: Vec<BlockRecord>,
    /// The dimension styles contained by the drawing.
    pub dim_styles: Vec<DimStyle>,
    /// The layers contained by the drawing.
    pub layers: Vec<Layer>,
    /// The line types contained by the drawing.
    pub line_types: Vec<LineType>,
    /// The visual styles contained by the drawing.
    pub styles: Vec<Style>,
    /// The user coordinate systems (UCS) contained by the drawing.
    pub ucs: Vec<Ucs>,
    /// The views contained by the drawing.
    pub views: Vec<View>,
    /// The view ports contained by the drawing.
    pub view_ports: Vec<ViewPort>,
}

// public implementation
impl Drawing {
    /// Creates a new empty `Drawing`.
    pub fn new() -> Self {
        Drawing {
            header: Header::new(),
            entities: vec![],
            app_ids: vec![],
            block_records: vec![],
            dim_styles: vec![],
            layers: vec![],
            line_types: vec![],
            styles: vec![],
            ucs: vec![],
            views: vec![],
            view_ports: vec![],
        }
    }
    /// Loads a `Drawing` from anything that implements the `Read` trait.
    pub fn load<T>(reader: T) -> DxfResult<Drawing>
        where T: Read {
        let reader = CodePairAsciiIter { reader: reader };
        let mut drawing = Drawing::new();
        let mut iter = PutBack::new(reader);
        try!(Drawing::read_sections(&mut drawing, &mut iter));
        match iter.next() {
            Some(Ok(CodePair { code: 0, value: CodePairValue::Str(ref s) })) if s == "EOF" => Ok(drawing),
            Some(Ok(pair)) => Err(DxfError::UnexpectedCodePair(pair, String::from("expected 0/EOF"))),
            Some(Err(e)) => Err(e),
            None => Ok(drawing),
        }
    }
    /// Loads a `Drawing` from disk, using a `BufReader`.
    pub fn load_file(file_name: &str) -> DxfResult<Drawing> {
        let path = Path::new(file_name);
        let file = try!(File::open(&path));
        let buf_reader = BufReader::new(file);
        Drawing::load(buf_reader)
    }
    /// Writes a `Drawing` to anything that implements the `Write` trait.
    pub fn save<T>(&self, writer: &mut T) -> DxfResult<()>
        where T: Write {
        let mut writer = CodePairAsciiWriter { writer: writer };
        try!(self.header.write(&mut writer));
        let write_handles = self.header.version >= AcadVersion::R13 || self.header.handles_enabled;
        try!(self.write_tables(write_handles, &mut writer));
        try!(self.write_entities(write_handles, &mut writer));
        // TODO: write other sections
        try!(writer.write_code_pair(&CodePair::new_str(0, "EOF")));
        Ok(())
    }
    /// Writes a `Drawing` to disk, using a `BufWriter`.
    pub fn save_file(&self, file_name: &str) -> DxfResult<()> {
        let path = Path::new(file_name);
        let file = try!(File::create(&path));
        let mut buf_writer = BufWriter::new(file);
        self.save(&mut buf_writer)
    }
}

// private implementation
impl Drawing {
    fn write_tables<T>(&self, write_handles: bool, writer: &mut CodePairAsciiWriter<T>) -> DxfResult<()>
        where T: Write {
        try!(writer.write_code_pair(&CodePair::new_str(0, "SECTION")));
        try!(writer.write_code_pair(&CodePair::new_str(2, "TABLES")));
        try!(write_tables(&self, write_handles, writer));
        try!(writer.write_code_pair(&CodePair::new_str(0, "ENDSEC")));
        Ok(())
    }
    fn write_entities<T>(&self, write_handles: bool, writer: &mut CodePairAsciiWriter<T>) -> DxfResult<()>
        where T: Write {
        try!(writer.write_code_pair(&CodePair::new_str(0, "SECTION")));
        try!(writer.write_code_pair(&CodePair::new_str(2, "ENTITIES")));
        for e in &self.entities {
            try!(e.write(&self.header.version, write_handles, writer));
        }

        try!(writer.write_code_pair(&CodePair::new_str(0, "ENDSEC")));
        Ok(())
    }
    fn read_sections<I>(drawing: &mut Drawing, iter: &mut PutBack<I>) -> DxfResult<()>
        where I: Iterator<Item = DxfResult<CodePair>> {
        loop {
            match iter.next() {
                Some(Ok(pair @ CodePair { code: 0, .. })) => {
                    match &*pair.value.assert_string() {
                        "EOF" => {
                            iter.put_back(Ok(pair));
                            break;
                        },
                        "SECTION" => {
                            match iter.next() {
                               Some(Ok(CodePair { code: 2, value: CodePairValue::Str(s) })) => {
                                    match &*s {
                                        "HEADER" => drawing.header = try!(Header::read(iter)),
                                        "ENTITIES" => try!(drawing.read_entities(iter)),
                                        "TABLES" => try!(drawing.read_tables(iter)),
                                        // TODO: read other sections
                                        _ => try!(Drawing::swallow_section(iter)),
                                    }

                                    match iter.next() {
                                        Some(Ok(CodePair { code: 0, value: CodePairValue::Str(ref s) })) if s == "ENDSEC" => (),
                                        Some(Ok(pair)) => return Err(DxfError::UnexpectedCodePair(pair, String::from("expected 0/ENDSEC"))),
                                        Some(Err(e)) => return Err(e),
                                        None => return Err(DxfError::UnexpectedEndOfInput),
                                    }
                                },
                                Some(Ok(pair)) => return Err(DxfError::UnexpectedCodePair(pair, String::from("expected 0/<section-name>"))),
                                Some(Err(e)) => return Err(e),
                                None => return Err(DxfError::UnexpectedEndOfInput),
                            }
                        },
                        _ => return Err(DxfError::UnexpectedCodePair(pair, String::from("expected 0/SECTION"))),
                    }
                },
                Some(Ok(pair)) => return Err(DxfError::UnexpectedCodePair(pair, String::from("expected 0/SECTION or 0/EOF"))),
                Some(Err(e)) => return Err(e),
                None => break, // ideally should have been 0/EOF
            }
        }

        Ok(())
    }
    fn swallow_section<I>(iter: &mut PutBack<I>) -> DxfResult<()>
        where I: Iterator<Item = DxfResult<CodePair>> {
        loop {
            match iter.next() {
                Some(Ok(pair)) => {
                    if pair.code == 0 && pair.value.assert_string() == "ENDSEC" {
                        iter.put_back(Ok(pair));
                        break;
                    }
                },
                Some(Err(e)) => return Err(e),
                None => break,
            }
        }

        Ok(())
    }
    fn read_entities<I>(&mut self, iter: &mut PutBack<I>) -> DxfResult<()>
        where I: Iterator<Item = DxfResult<CodePair>> {
        let mut iter = PutBack::new(EntityIter { iter: iter });
        loop {
            match iter.next() {
                Some(Entity { ref common, specific: EntityType::Insert(ref ins) }) if ins.has_attributes => {
                    let mut ins = ins.clone(); // 12 fields
                    loop {
                        match iter.next() {
                            Some(Entity { specific: EntityType::Attribute(att), .. }) => ins.attributes.push(att),
                            Some(ent) => {
                                // stop gathering on any non-ATTRIBUTE
                                iter.put_back(ent);
                                break;
                            },
                            None => break,
                        }
                    }

                    try!(Drawing::swallow_seqend(&mut iter));

                    // and finally keep the INSERT
                    self.entities.push(Entity {
                        common: common.clone(), // 18 fields
                        specific: EntityType::Insert(ins),
                    })
                },
                Some(Entity { common, specific: EntityType::Polyline(poly) }) => {
                    let mut poly = poly.clone(); // 13 fields
                    loop {
                        match iter.next() {
                            Some(Entity { specific: EntityType::Vertex(vertex), .. }) => poly.vertices.push(vertex),
                            Some(ent) => {
                                // stop gathering on any non-VERTEX
                                iter.put_back(ent);
                                break;
                            },
                            None => break,
                        }
                    }

                    try!(Drawing::swallow_seqend(&mut iter));

                    // and finally keep the POLYLINE
                    self.entities.push(Entity {
                        common: common.clone(), // 18 fields
                        specific: EntityType::Polyline(poly),
                    });
                },
                Some(entity) => self.entities.push(entity),
                None => break,
            }
        }

        Ok(())
    }
    fn swallow_seqend<I>(iter: &mut PutBack<I>) -> DxfResult<()>
        where I: Iterator<Item = Entity> {
        match iter.next() {
            Some(Entity { specific: EntityType::Seqend(_), .. }) => (),
            Some(ent) => iter.put_back(ent),
            None => (),
        }

        Ok(())
    }
    fn read_tables<I>(&mut self, iter: &mut PutBack<I>) -> DxfResult<()>
        where I: Iterator<Item = DxfResult<CodePair>> {
        loop {
            match iter.next() {
                Some(Ok(pair)) => {
                    if pair.code == 0 {
                        match &*pair.value.assert_string() {
                            "ENDSEC" => {
                                iter.put_back(Ok(pair));
                                break;
                            },
                            "TABLE" => try!(read_specific_table(self, iter)),
                            _ => return Err(DxfError::UnexpectedCodePair(pair, String::new())),
                        }
                    }
                    else {
                        return Err(DxfError::UnexpectedCodePair(pair, String::new()));
                    }
                },
                Some(Err(e)) => return Err(e),
                None => return Err(DxfError::UnexpectedEndOfInput),
            }
        }

        Ok(())
    }
    #[doc(hidden)]
    pub fn swallow_table<I>(iter: &mut PutBack<I>) -> DxfResult<()>
        where I: Iterator<Item = DxfResult<CodePair>> {
        loop {
            match iter.next() {
                Some(Ok(pair)) => {
                    if pair.code == 0 {
                        match &*pair.value.assert_string() {
                            "TABLE" | "ENDSEC" | "ENDTAB" => {
                                iter.put_back(Ok(pair));
                                break;
                            },
                            _ => (), // swallow the code pair
                        }
                    }
                }
                Some(Err(e)) => return Err(e),
                None => return Err(DxfError::UnexpectedEndOfInput),
            }
        }

        Ok(())
    }
}