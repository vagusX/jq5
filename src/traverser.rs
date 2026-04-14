// Based on https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/tools/jq5
// Original: Copyright 2021 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use json5format::*;
use std::mem::discriminant;

/// A recursive function that takes in two references to `json5format::Value`s,
/// and modifies the second one to incorporate comments contained in the first.
///
/// The function does a depth-first search of the JSON5 tree structure,
/// transferring comments for a value X in `json5_val` to a value Y in
/// `jq_output_val` if both:
///   a) X and Y have the same path from the root of the tree
///   b) X and Y are the same `json5format::Value` variant
fn fill_comments_helper(json5_val: &Value, jq_output_val: &mut Value) -> Result<(), anyhow::Error> {
    match json5_val {
        Value::Object { val, comments } => match jq_output_val {
            Value::Object {
                val: out_val,
                comments: out_comments,
            } => {
                *out_comments = comments.clone();
                *out_val.trailing_comments_mut() = val.trailing_comments().clone();
                let mut out_properties: Vec<_> = out_val.properties_mut().collect();
                for property in val.properties() {
                    let index = out_properties
                        .iter()
                        .position(|p_output| p_output.name() == property.name());
                    if let Some(i) = index {
                        if discriminant(&*property.value()) == discriminant(&*out_properties[i].value())
                        {
                            fill_comments_helper(
                                &property.value(),
                                &mut out_properties[i].value_mut(),
                            )?;
                        }
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "fill_comments_helper was called on mismatched Value variants.\njson5_val was: {:?}\njq_output_val was: {:?}",
                    json5_val,
                    jq_output_val
                ));
            }
        },
        Value::Array { val, comments } => match jq_output_val {
            Value::Array {
                val: out_val,
                comments: out_comments,
            } => {
                *out_comments = comments.clone();
                *out_val.trailing_comments_mut() = val.trailing_comments().clone();
                for (sub_val, mut out_sub_val) in val.items().zip(out_val.items_mut()) {
                    if discriminant(&(*sub_val)) == discriminant(&(*out_sub_val)) {
                        fill_comments_helper(&sub_val, &mut out_sub_val)?;
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "fill_comments_helper was called on mismatched Value variants.\njson5_val was: {:?}\njq_output_val was: {:?}",
                    json5_val,
                    jq_output_val
                ));
            }
        },
        Value::Primitive { comments, .. } => match jq_output_val {
            Value::Primitive {
                comments: out_comments,
                ..
            } => {
                *out_comments = comments.clone();
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "fill_comments_helper was called on mismatched Value variants.\njson5_val was: {:?}\njq_output_val was: {:?}",
                    json5_val,
                    jq_output_val
                ));
            }
        },
    };
    Ok(())
}

/// Transfers the comments at the bottom of the document below the json5 object
/// and calls `fill_comments_helper` to transfer the rest of the comments.
#[inline]
pub(crate) fn fill_comments(
    json5_content: &Array,
    jq_output_content: &mut Array,
) -> Result<(), anyhow::Error> {
    *jq_output_content.trailing_comments_mut() = json5_content.trailing_comments().clone();
    fill_comments_helper(
        &json5_content.items().next().unwrap(),
        &mut jq_output_content.items_mut().next().unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_comments_handles_changed_field_type() {
        let json5_original = String::from(
            r##"{
// Foo
"offer": [
{
// Bar
"directory": "input",
"from": "self",
// Baz
"to": [ "#pwrbtn-monitor" ]
},
{
"protocol": "fuchsia.hardware.power.statecontrol.Admin",
"from": "self",
"to": [ "#pwrbtn-monitor" ]
}
]
}"##,
        );
        let json5_target = String::from(
            r##"{
"offer": [
{
"directory": "input",
"from": "self",
"to": "#pwrbtn-monitor"
},
{
"protocol": "fuchsia.hardware.power.statecontrol.Admin",
"from": "self",
"to": "#pwrbtn-monitor"
}
]
}"##,
        );
        let expected_outcome = String::from(
            r##"{
// Foo
"offer": [
{
// Bar
"directory": "input",
"from": "self",
"to": "#pwrbtn-monitor"
},
{
"protocol": "fuchsia.hardware.power.statecontrol.Admin",
"from": "self",
"to": "#pwrbtn-monitor"
}
]
}"##,
        );

        let parsed_json5_original =
            json5format::ParsedDocument::from_str(&json5_original[..], None).unwrap();
        let mut parsed_json5_target =
            json5format::ParsedDocument::from_str(&json5_target[..], None).unwrap();
        let parsed_expected_outcome =
            json5format::ParsedDocument::from_str(&expected_outcome[..], None).unwrap();

        fill_comments(&parsed_json5_original.content, &mut parsed_json5_target.content).unwrap();

        let format = json5format::Json5Format::new().unwrap();
        assert_eq!(
            format.to_string(&parsed_expected_outcome).unwrap(),
            format.to_string(&parsed_json5_target).unwrap()
        );
    }

    #[test]
    fn simple_transfer_object1() {
        let json5_original = String::from(
            r##"
//Comment at
//Beginning of
//Document
{
// Foo
hello: 'world',
// Bar
yoinks: "everybody", //Baz
//Contained Comment (at end of object)
}
"##,
        );
        let json5_target = String::from(
            r##"
{
hello: 'world',
yoinks: "everybody",
}
"##,
        );

        let parsed_json5_original =
            json5format::ParsedDocument::from_str(&json5_original[..], None).unwrap();
        let mut parsed_json5_target =
            json5format::ParsedDocument::from_str(&json5_target[..], None).unwrap();

        fill_comments(&parsed_json5_original.content, &mut parsed_json5_target.content).unwrap();

        let format = json5format::Json5Format::new().unwrap();
        assert_eq!(
            format.to_string(&parsed_json5_original).unwrap(),
            format.to_string(&parsed_json5_target).unwrap()
        );
    }

    #[test]
    fn simple_transfer_object3() {
        let json5_original = String::from(
            r##"
//Comment at
//Beginning of
//Document
{
// Foo
hello: 'world',
// Bar
yoinks: 'everybody', //Baz
//This comment will be deleted since the field does not appear in the target
removed_in_target: "Away I must go!",
//Contained Comment (at end of object)
}
"##,
        );
        let json5_target = String::from(
            r##"
{
hello: 'world',
yoinks: 'everybody',
//This comment will stay, since there is no node in the original that overrides this node.
addition_in_target: 'Nice to meet you!',
}
"##,
        );

        let parsed_json5_original =
            json5format::ParsedDocument::from_str(&json5_original[..], None).unwrap();
        let mut parsed_json5_target =
            json5format::ParsedDocument::from_str(&json5_target[..], None).unwrap();

        fill_comments(&parsed_json5_original.content, &mut parsed_json5_target.content).unwrap();

        let format = json5format::Json5Format::new().unwrap();
        let expected_outcome = String::from(
            r##"
//Comment at
//Beginning of
//Document
{
// Foo
hello: 'world',
// Bar
yoinks: 'everybody', //Baz
//This comment will stay, since there is no node in the original that overrides this node.
addition_in_target: 'Nice to meet you!',
//Contained Comment (at end of object)
}
"##,
        );

        assert_eq!(
            format.to_string(&parsed_json5_target).unwrap(),
            format
                .to_string(
                    &json5format::ParsedDocument::from_str(&expected_outcome[..], None).unwrap()
                )
                .unwrap()
        );
    }

    #[test]
    fn recursive_transfer() {
        let json5_original = String::from(
            r##"
//Comment at beginning.
{
//Foo1
"Foo1": "Bar1",
//Array
"arr": [
//Primitive in array. Even though the actual value changes, this comment
//gets carried over since the same path from root exists in the target.
"Baz",
//Object in array
{
//Foo2
"Foo2": "Bar2",
//Field entry not present in target. This comment will not carry over.
"deleted_field2": "bye, again!"
//Comment at end of second object.
}
//Comment at end of array.
],
//Field entry not present in target. This comment will not carry over.
"deleted_field1": "bye, world!"
//Comment at end of first object.
}
// Comment at end of document.
"##,
        );

        let json5_target = String::from(
            r##"
//This comment will be overriden since its path exists in the original.
{
//This comment will also be overriden.
"Foo1": "Bar1",
//This comment will also be overriden.
"arr": [
//This comment will also be overriden (even though the value changed).
"BazBaz",
//This comment will also be overriden.
{
//This comment will also be overriden.
"Foo2": "Bar2",
//Comment on new_field2 (will stay).
"new_field2": "hello, again!"
},
//Comment on new array entry, 0 (will stay).
0
],
//Comment on new_field1 (will stay).
"new_field1": "hello, world!"
}
"##,
        );

        let parsed_json5_original =
            json5format::ParsedDocument::from_str(&json5_original[..], None).unwrap();
        let mut parsed_json5_target =
            json5format::ParsedDocument::from_str(&json5_target[..], None).unwrap();

        fill_comments(&parsed_json5_original.content, &mut parsed_json5_target.content).unwrap();

        let format = json5format::Json5Format::new().unwrap();
        let expected_outcome = String::from(
            r##"
//Comment at beginning.
{
//Foo1
"Foo1": "Bar1",
//Array
"arr": [
//Primitive in array. Even though the actual value changes, this comment
//gets carried over since the same path from root exists in the target.
"BazBaz",
//Object in array
{
//Foo2
"Foo2": "Bar2",
//Comment on new_field2 (will stay).
"new_field2": "hello, again!"
//Comment at end of second object.
},
//Comment on new array entry, 0 (will stay).
0
//Comment at end of array.
],
//Comment on new_field1 (will stay).
"new_field1": "hello, world!"
//Comment at end of first object.
}
// Comment at end of document.
"##,
        );

        assert_eq!(
            format.to_string(&parsed_json5_target).unwrap(),
            format
                .to_string(
                    &json5format::ParsedDocument::from_str(&expected_outcome[..], None).unwrap()
                )
                .unwrap()
        );
    }
}
