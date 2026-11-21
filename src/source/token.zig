const std = @import("std");

pub const TokenKind = enum {
    script_open,        // <script ...>
    script_close,       // </script>
    script_raw,         // script body
    state_open,         // <state>
    state_close,        // </state>
    state_raw,          // state body
    style_open,         // <style>
    style_close,        // </style>
    style_raw,          // style body
    tag_open,           // <tagname
    tag_close,          // </tagname>
    tag_end,            // >
    tag_self_close,     // />
    attribute_name,     // attr
    attribute_value,    // "val"
    event_name,         // click
    event_expr,         // {count++}
    interpolation,      // {greeting()}
    text,               // raw text
    eof,
};

pub const Token = struct {
    kind: TokenKind,
    text: []const u8,
    line: usize,
    col: usize,
};
