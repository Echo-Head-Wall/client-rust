/*
 * Created on Wed May 05 2021
 *
 * Copyright (c) 2021 Sayan Nandan <nandansayan@outlook.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *    http://www.apache.org/licenses/LICENSE-2.0
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

//! This module provides methods to deserialize an incoming response packet

use crate::terrapipe::RespCode;

/// A response datagroup
///
/// This contains all the elements returned by a certain action. So let's say you did
/// something like `MGET x y`, then the values of x and y will be in a single datagroup.
pub type DataGroup = Vec<DataType>;

/// A data type as defined by the Terrapipe protocol
///
///
/// Every variant stays in an `Option` for convenience while parsing. It's like we first
/// create a `Variant(None)` variant. Then we read the data which corresponds to it, and then we
/// replace `None` with the appropriate object. When we first detect the type, we use this as a way of matching
/// avoiding duplication by writing another `DataType` enum
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum DataType {
    /// A string value
    Str(String),
    /// A response code (it is kept as `String` for "other error" types)
    RespCode(RespCode),
    /// An unsigned 64-bit integer, equivalent to an `u64`
    UnsignedInt(u64),
}

#[non_exhaustive]
enum _DataType {
    Str(Option<String>),
    RespCode(Option<RespCode>),
    UnsignedInt(Option<Result<u64, std::num::ParseIntError>>),
}

/// Errors that may occur while parsing responses from the server
///
/// Every variant, except `Incomplete` has an `usize` field, which is used to advance the
/// buffer
#[derive(Debug, PartialEq)]
pub enum ClientResult {
    /// The response was Invalid
    InvalidResponse,
    /// The response is a valid response and has been parsed into a vector of datagroups
    PipelinedResponse(Vec<DataGroup>, usize),
    /// The response is a valid response and has been parsed into a datagroup
    SimpleResponse(DataGroup, usize),
    /// A single element in a datagroup (please note that this is a client abstraction)
    ResponseItem(DataType, usize),
    /// The response was empty, which means that the remote end closed the connection
    Empty,
    /// The response is incomplete
    Incomplete,
    /// The server returned data, but we couldn't parse it
    ParseError,
}

/// Parse a response packet
pub fn parse(buf: &[u8]) -> ClientResult {
    if buf.len() < 6 {
        // A packet that has less than 6 characters? Nonsense!
        return ClientResult::Incomplete;
    }
    /*
    We first get the metaframe, which looks something like:
    ```
    #<numchars_in_next_line>\n
    *<num_of_datagroups>\n
    ```
    */
    let mut pos = 0;
    if buf[pos] != b'#' {
        return ClientResult::InvalidResponse;
    } else {
        pos += 1;
    }
    let next_line = match read_line_and_return_next_line(&mut pos, &buf) {
        Some(line) => line,
        None => {
            // This is incomplete
            return ClientResult::Incomplete;
        }
    };
    pos += 1; // Skip LF
              // Find out the number of actions that we have to do
    let mut action_size = 0usize;
    if next_line[0] == b'*' {
        let mut line_iter = next_line.into_iter().skip(1).peekable();
        while let Some(dig) = line_iter.next() {
            let curdig: usize = match dig.checked_sub(48) {
                Some(dig) => {
                    if dig > 9 {
                        return ClientResult::InvalidResponse;
                    } else {
                        dig.into()
                    }
                }
                None => return ClientResult::InvalidResponse,
            };
            action_size = (action_size * 10) + curdig;
        }
    // This line gives us the number of actions
    } else {
        return ClientResult::InvalidResponse;
    }
    let mut items: Vec<DataGroup> = Vec::with_capacity(action_size);
    while pos < buf.len() && items.len() <= action_size {
        match buf[pos] {
            b'#' => {
                pos += 1; // Skip '#'
                let next_line = match read_line_and_return_next_line(&mut pos, &buf) {
                    Some(line) => line,
                    None => {
                        // This is incomplete
                        return ClientResult::Incomplete;
                    }
                }; // Now we have the current line
                pos += 1; // Skip the newline
                          // Move the cursor ahead by the number of bytes that we just read
                          // Let us check the current char
                match next_line[0] {
                    b'&' => {
                        // This is an array
                        // Now let us parse the array size
                        let mut current_array_size = 0usize;
                        let mut linepos = 1; // Skip the '&' character
                        while linepos < next_line.len() {
                            let curdg: usize = match next_line[linepos].checked_sub(48) {
                                Some(dig) => {
                                    if dig > 9 {
                                        // If `dig` is greater than 9, then the current
                                        // UTF-8 char isn't a number
                                        return ClientResult::InvalidResponse;
                                    } else {
                                        dig.into()
                                    }
                                }
                                None => return ClientResult::InvalidResponse,
                            };
                            current_array_size = (current_array_size * 10) + curdg; // Increment the size
                            linepos += 1; // Move the position ahead, since we just read another char
                        }
                        // Now we know the array size, good!
                        let mut actiongroup: Vec<DataType> = Vec::with_capacity(current_array_size);
                        // Let's loop over to get the elements till the size of this array
                        while pos < buf.len() && actiongroup.len() < current_array_size {
                            let mut element_size = 0usize;
                            let datatype = match buf[pos] {
                                b'+' => _DataType::Str(None),
                                b'!' => _DataType::RespCode(None),
                                b':' => _DataType::UnsignedInt(None),
                                x @ _ => unimplemented!("Type '{}' not implemented", char::from(x)),
                            };
                            pos += 1; // We've got the tsymbol above, so skip it
                            while pos < buf.len() && buf[pos] != b'\n' {
                                let curdig: usize = match buf[pos].checked_sub(48) {
                                    Some(dig) => {
                                        if dig > 9 {
                                            // If `dig` is greater than 9, then the current
                                            // UTF-8 char isn't a number
                                            return ClientResult::InvalidResponse;
                                        } else {
                                            dig.into()
                                        }
                                    }
                                    None => return ClientResult::InvalidResponse,
                                };
                                element_size = (element_size * 10) + curdig; // Increment the size
                                pos += 1; // Move the position ahead, since we just read another char
                            }
                            pos += 1;
                            // We now know the item size
                            let mut value = String::with_capacity(element_size);
                            let extracted = match buf.get(pos..pos + element_size) {
                                Some(s) => s,
                                None => return ClientResult::Incomplete,
                            };
                            pos += element_size; // Move the position ahead
                            value.push_str(&String::from_utf8_lossy(extracted));
                            pos += 1; // Skip the newline
                            actiongroup.push(match datatype {
                                _DataType::Str(_) => DataType::Str(value),
                                _DataType::RespCode(_) => {
                                    DataType::RespCode(RespCode::from_str(&value))
                                }
                                _DataType::UnsignedInt(_) => {
                                    if let Ok(unsigned_int64) = value.parse() {
                                        DataType::UnsignedInt(unsigned_int64)
                                    } else {
                                        return ClientResult::ParseError;
                                    }
                                }
                            });
                        }
                        items.push(actiongroup);
                    }
                    _ => return ClientResult::InvalidResponse,
                }
                continue;
            }
            _ => {
                // Since the variant '#' would does all the array
                // parsing business, we should never reach here unless
                // the packet is invalid
                return ClientResult::InvalidResponse;
            }
        }
    }
    if buf.get(pos).is_none() {
        if items.len() == action_size {
            if items.len() == 1 {
                if items[0].len() == 1 {
                    // Single item returned, so we can return this as ClientResult::ResponseItem
                    ClientResult::ResponseItem(items.swap_remove(0).swap_remove(0), pos)
                } else {
                    // More than one time returned, so we can return this as ClientResult::Response
                    ClientResult::SimpleResponse(items.swap_remove(0), pos)
                }
            } else {
                ClientResult::PipelinedResponse(items, pos)
            }
        } else {
            // Since the number of items we got is not equal to the action size - not all data was
            // transferred
            ClientResult::Incomplete
        }
    } else {
        // Either more data was sent or some data was missing
        ClientResult::InvalidResponse
    }
}
/// Read a size line and return the following line
///
/// This reads a line that begins with the number, i.e make sure that
/// the **`#` character is skipped**
///
fn read_line_and_return_next_line<'a>(pos: &mut usize, buf: &'a [u8]) -> Option<&'a [u8]> {
    let mut next_line_size = 0usize;
    while pos < &mut buf.len() && buf[*pos] != b'\n' {
        // 48 is the UTF-8 code for '0'
        let curdig: usize = match buf[*pos].checked_sub(48) {
            Some(dig) => {
                if dig > 9 {
                    // If `dig` is greater than 9, then the current
                    // UTF-8 char isn't a number
                    return None;
                } else {
                    dig.into()
                }
            }
            None => return None,
        };
        next_line_size = (next_line_size * 10) + curdig; // Increment the size
        *pos += 1; // Move the position ahead, since we just read another char
    }
    *pos += 1; // Skip the newline
               // We now know the size of the next line
    let next_line = match buf.get(*pos..*pos + next_line_size) {
        Some(line) => line,
        None => {
            // This is incomplete
            return None;
        }
    }; // Now we have the current line
       // Move the cursor ahead by the number of bytes that we just read
    *pos += next_line_size;
    Some(next_line)
}

#[cfg(test)]
#[test]
fn test_deserializer_responseitem() {
    let res = "#2\n*1\n#2\n&1\n+4\nHEY!\n".as_bytes().to_owned();
    assert_eq!(
        parse(&res),
        ClientResult::ResponseItem(DataType::Str("HEY!".to_owned()), res.len())
    );
    let res = "#2\n*1\n#2\n&1\n!1\n0\n".as_bytes().to_owned();
    assert_eq!(
        parse(&res),
        ClientResult::ResponseItem(DataType::RespCode(RespCode::Okay), res.len())
    );
}

#[cfg(test)]
#[test]
fn test_deserializer_simple_response() {
    let res = "#2\n*1\n#2\n&5\n!1\n1\n!1\n0\n+5\nsayan\n+2\nis\n+4\nbusy\n"
        .as_bytes()
        .to_owned();
    let ret = parse(&res);
    assert_eq!(
        ret,
        ClientResult::SimpleResponse(
            vec![
                DataType::RespCode(RespCode::NotFound),
                DataType::RespCode(RespCode::Okay),
                DataType::Str("sayan".to_owned()),
                DataType::Str("is".to_owned()),
                DataType::Str("busy".to_owned())
            ],
            res.len()
        )
    );
    if let ClientResult::SimpleResponse(ret, _) = ret {
        for val in ret {
            let _ = format!("{:?}", val);
        }
    } else {
        panic!("Expected a SimpleResponse with a single datagroup")
    }
}
