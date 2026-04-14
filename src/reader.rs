// Based on https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/tools/jq5
// Original: Copyright 2021 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use json5format::*;
use serde_json::Value;
use std::fs::File;
use std::path::PathBuf;

/// Processes `json5_string`, a string representing a JSON5 object, and returns
/// a tuple of two elements where the first element is a `ParsedDocument`
/// representation of the JSON5 object and the second element is a string
/// representing the JSON5 object without comments (as plain JSON).
pub(crate) fn read_json5(json5_string: String) -> Result<(ParsedDocument, String), anyhow::Error> {
    let object_as_json_string = serde_json5::from_str::<Value>(&json5_string)?.to_string();
    let deserialized_object = json5format::ParsedDocument::from_string(json5_string, None)?;
    Ok((deserialized_object, object_as_json_string))
}

/// Calls `read_json5` on the contents of the specified file.
pub(crate) fn read_json5_fromfile(
    file: &PathBuf,
) -> Result<(ParsedDocument, String), anyhow::Error> {
    let path = file.as_path();
    read_json5_from_input(&mut File::open(path)?)
}

/// Calls `read_json5` on the data from an object with a `Read` implementation.
pub(crate) fn read_json5_from_input(
    input: &mut (impl std::io::Read + Sized),
) -> Result<(ParsedDocument, String), anyhow::Error> {
    let mut buffer = String::new();
    input.read_to_string(&mut buffer)?;
    read_json5(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn serialized_outputs_equivalent_2() {
        let simple_json5 = String::from(
            r##"
{
// Foo
hello: 'world',
// Bar
yoinks: "scoob",
}
"##,
        );
        let simple_json = String::from(r#"{"hello":"world","yoinks":"scoob"}"#);
        let parsed_json5 =
            json5format::ParsedDocument::from_str(&(read_json5(simple_json5).unwrap().1)[..], None)
                .unwrap();
        let parsed_json = json5format::ParsedDocument::from_str(&simple_json[..], None).unwrap();
        let format = json5format::Json5Format::new().unwrap();
        assert_eq!(
            format.to_string(&parsed_json5).unwrap(),
            format.to_string(&parsed_json).unwrap()
        );
    }

    #[test]
    fn read_json5_from_input_1() {
        let json5_string = String::from(
            r##"
{
// Foo
hello: 'world',
// Bar
yoinks: 'scoob',
}
"##,
        );
        let mut mock_stdin = Cursor::new(json5_string.as_bytes());
        let result_from_input = read_json5_from_input(&mut mock_stdin).unwrap();
        let result_from_string = read_json5(json5_string).unwrap();
        let format = Json5Format::new().unwrap();
        assert_eq!(
            format.to_string(&result_from_input.0).unwrap(),
            format.to_string(&result_from_string.0).unwrap()
        );
        assert_eq!(&result_from_input.1, &result_from_string.1);
    }
}
