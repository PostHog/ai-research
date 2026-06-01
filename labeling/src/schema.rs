#![allow(dead_code)]

use std::borrow::Cow;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum EventType {
    DomContentLoaded = 0,
    Load = 1,
    FullSnapshot = 2,
    IncrementalSnapshot = 3,
    Meta = 4,
    Custom = 5,
    Plugin = 6,
}

impl EventType {
    pub const fn from_u8(n: u8) -> Option<Self> {
        Some(match n {
            0 => Self::DomContentLoaded,
            1 => Self::Load,
            2 => Self::FullSnapshot,
            3 => Self::IncrementalSnapshot,
            4 => Self::Meta,
            5 => Self::Custom,
            6 => Self::Plugin,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum IncrementalSource {
    Mutation = 0,
    MouseMove = 1,
    MouseInteraction = 2,
    Scroll = 3,
    ViewportResize = 4,
    Input = 5,
    TouchMove = 6,
    MediaInteraction = 7,
    StyleSheetRule = 8,
    CanvasMutation = 9,
    Font = 10,
    Log = 11,
    Drag = 12,
    StyleDeclaration = 13,
    Selection = 14,
    AdoptedStyleSheet = 15,
    CustomElement = 16,
}

impl IncrementalSource {
    pub const fn from_u8(n: u8) -> Option<Self> {
        Some(match n {
            0 => Self::Mutation,
            1 => Self::MouseMove,
            2 => Self::MouseInteraction,
            3 => Self::Scroll,
            4 => Self::ViewportResize,
            5 => Self::Input,
            6 => Self::TouchMove,
            7 => Self::MediaInteraction,
            8 => Self::StyleSheetRule,
            9 => Self::CanvasMutation,
            10 => Self::Font,
            11 => Self::Log,
            12 => Self::Drag,
            13 => Self::StyleDeclaration,
            14 => Self::Selection,
            15 => Self::AdoptedStyleSheet,
            16 => Self::CustomElement,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum NodeType {
    Document = 0,
    DocumentType = 1,
    Element = 2,
    Text = 3,
    Cdata = 4,
    Comment = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum MouseInteractionKind {
    MouseUp = 0,
    MouseDown = 1,
    Click = 2,
    ContextMenu = 3,
    DblClick = 4,
    Focus = 5,
    Blur = 6,
    TouchStart = 7,
    TouchMoveDeparted = 8,
    TouchEnd = 9,
    TouchCancel = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum PointerType {
    Mouse = 0,
    Pen = 1,
    Touch = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum MediaInteractionKind {
    Play = 0,
    Pause = 1,
    Seeked = 2,
    VolumeChange = 3,
    RateChange = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum CanvasContext {
    Ctx2d = 0,
    WebGl = 1,
    WebGl2 = 2,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Event<D> {
    #[serde(rename = "type")]
    pub ty: u8,
    pub timestamp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<i64>,
    pub data: D,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventCompressed<'a, D> {
    #[serde(rename = "type")]
    pub ty: u8,
    pub timestamp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<i64>,
    #[serde(borrow)]
    pub cv: Cow<'a, str>,
    pub data: D,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EmptyData {}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetaData<'a> {
    #[serde(borrow)]
    pub href: Cow<'a, str>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CustomEventData<'a> {
    #[serde(borrow)]
    pub tag: Cow<'a, str>,
    #[serde(default = "null_value")]
    pub payload: simd_json::OwnedValue,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginEventData<'a> {
    #[serde(borrow)]
    pub plugin: Cow<'a, str>,
    pub payload: simd_json::OwnedValue,
}

fn null_value() -> simd_json::OwnedValue {
    simd_json::OwnedValue::Static(simd_json::StaticNode::Null)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullSnapshotData<'a> {
    #[serde(borrow)]
    pub node: SerializedNodeWithId<'a>,
    pub initial_offset: InitialOffset,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InitialOffset {
    pub top: f64,
    pub left: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SerializedNodeWithId<'a> {
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_shadow_host: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_shadow: Option<bool>,
    #[serde(borrow, flatten)]
    pub node: SerializedNode<'a>,
}

#[derive(Debug, Clone)]
pub enum SerializedNode<'a> {
    Document(DocumentNode<'a>),
    DocumentType(DocumentTypeNode<'a>),
    Element(ElementNode<'a>),
    Text(TextNode<'a>),
    Cdata(CdataNode),
    Comment(CommentNode<'a>),
}

// Avoids serde's untagged-enum trial-and-error: dispatch on `type`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeHelper<'a> {
    #[serde(rename = "type")]
    ty: NodeType,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    tag_name: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    attributes: Option<FxHashMap<Cow<'a, str>, AttrValue<'a>>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    child_nodes: Option<Vec<SerializedNodeWithId<'a>>>,
    #[serde(rename = "isSVG", default, skip_serializing_if = "Option::is_none")]
    is_svg: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    need_block: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is_custom: Option<bool>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    text_content: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is_style: Option<bool>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    name: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    public_id: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    system_id: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    compat_mode: Option<Cow<'a, str>>,
}

impl<'de: 'a, 'a> Deserialize<'de> for SerializedNode<'a> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let h = NodeHelper::<'a>::deserialize(d)?;
        Ok(match h.ty {
            NodeType::Document => SerializedNode::Document(DocumentNode {
                child_nodes: h.child_nodes.unwrap_or_default(),
                compat_mode: h.compat_mode,
            }),
            NodeType::DocumentType => SerializedNode::DocumentType(DocumentTypeNode {
                name: h.name.unwrap_or(Cow::Borrowed("")),
                public_id: h.public_id.unwrap_or(Cow::Borrowed("")),
                system_id: h.system_id.unwrap_or(Cow::Borrowed("")),
            }),
            NodeType::Element => SerializedNode::Element(ElementNode {
                tag_name: h.tag_name.unwrap_or(Cow::Borrowed("")),
                attributes: h.attributes.unwrap_or_default(),
                child_nodes: h.child_nodes.unwrap_or_default(),
                is_svg: h.is_svg,
                need_block: h.need_block,
                is_custom: h.is_custom,
            }),
            NodeType::Text => SerializedNode::Text(TextNode {
                text_content: h.text_content.unwrap_or(Cow::Borrowed("")),
                is_style: h.is_style,
            }),
            NodeType::Cdata => SerializedNode::Cdata(CdataNode {}),
            NodeType::Comment => SerializedNode::Comment(CommentNode {
                text_content: h.text_content.unwrap_or(Cow::Borrowed("")),
            }),
        })
    }
}

impl<'a> Serialize for SerializedNode<'a> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let h: NodeHelper<'_> = match self {
            SerializedNode::Document(v) => NodeHelper {
                ty: NodeType::Document,
                child_nodes: Some(v.child_nodes.clone()),
                compat_mode: v.compat_mode.clone(),
                ..empty_helper(NodeType::Document)
            },
            SerializedNode::DocumentType(v) => NodeHelper {
                ty: NodeType::DocumentType,
                name: Some(v.name.clone()),
                public_id: Some(v.public_id.clone()),
                system_id: Some(v.system_id.clone()),
                ..empty_helper(NodeType::DocumentType)
            },
            SerializedNode::Element(v) => NodeHelper {
                ty: NodeType::Element,
                tag_name: Some(v.tag_name.clone()),
                attributes: Some(v.attributes.clone()),
                child_nodes: Some(v.child_nodes.clone()),
                is_svg: v.is_svg,
                need_block: v.need_block,
                is_custom: v.is_custom,
                ..empty_helper(NodeType::Element)
            },
            SerializedNode::Text(v) => NodeHelper {
                ty: NodeType::Text,
                text_content: Some(v.text_content.clone()),
                is_style: v.is_style,
                ..empty_helper(NodeType::Text)
            },
            SerializedNode::Cdata(_) => empty_helper(NodeType::Cdata),
            SerializedNode::Comment(v) => NodeHelper {
                ty: NodeType::Comment,
                text_content: Some(v.text_content.clone()),
                ..empty_helper(NodeType::Comment)
            },
        };
        h.serialize(s)
    }
}

fn empty_helper<'a>(ty: NodeType) -> NodeHelper<'a> {
    NodeHelper {
        ty,
        tag_name: None,
        attributes: None,
        child_nodes: None,
        is_svg: None,
        need_block: None,
        is_custom: None,
        text_content: None,
        is_style: None,
        name: None,
        public_id: None,
        system_id: None,
        compat_mode: None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentNode<'a> {
    pub child_nodes: Vec<SerializedNodeWithId<'a>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compat_mode: Option<Cow<'a, str>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTypeNode<'a> {
    pub name: Cow<'a, str>,
    pub public_id: Cow<'a, str>,
    pub system_id: Cow<'a, str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementNode<'a> {
    pub tag_name: Cow<'a, str>,
    pub attributes: FxHashMap<Cow<'a, str>, AttrValue<'a>>,
    pub child_nodes: Vec<SerializedNodeWithId<'a>>,
    #[serde(rename = "isSVG", skip_serializing_if = "Option::is_none")]
    pub is_svg: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_block: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_custom: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextNode<'a> {
    pub text_content: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_style: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CdataNode {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentNode<'a> {
    pub text_content: Cow<'a, str>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AttrValue<'a> {
    #[serde(borrow)]
    Str(Cow<'a, str>),
    Num(f64),
    Bool(bool),
    Null,
    #[serde(borrow)]
    Obj(FxHashMap<Cow<'a, str>, AttrValue<'a>>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MutationData<'a> {
    pub source: IncrementalSource, // == Mutation (0)
    #[serde(borrow, default)]
    pub texts: Vec<TextMutation<'a>>,
    #[serde(borrow, default)]
    pub attributes: Vec<AttributeMutation<'a>>,
    #[serde(default)]
    pub removes: Vec<RemovedNodeMutation>,
    #[serde(borrow, default)]
    pub adds: Vec<AddedNodeMutation<'a>>,
    #[serde(
        default,
        rename = "isAttachIframe",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_attach_iframe: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MutationDataCompressed<'a> {
    pub source: IncrementalSource, // == Mutation
    #[serde(borrow, default)]
    pub texts: Cow<'a, str>,
    #[serde(borrow, default)]
    pub attributes: Cow<'a, str>,
    #[serde(borrow, default)]
    pub removes: Cow<'a, str>,
    #[serde(borrow, default)]
    pub adds: Cow<'a, str>,
    #[serde(
        default,
        rename = "isAttachIframe",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_attach_iframe: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextMutation<'a> {
    pub id: i64,
    #[serde(borrow)]
    pub value: Option<Cow<'a, str>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttributeMutation<'a> {
    pub id: i64,
    #[serde(borrow)]
    pub attributes: FxHashMap<Cow<'a, str>, AttrValue<'a>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedNodeMutation {
    pub parent_id: i64,
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_shadow: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedNodeMutation<'a> {
    pub parent_id: i64,
    #[serde(default)]
    pub previous_id: Option<i64>,
    pub next_id: Option<i64>,
    #[serde(borrow)]
    pub node: SerializedNodeWithId<'a>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MouseMoveData {
    pub source: IncrementalSource,
    pub positions: Vec<MousePosition>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MousePosition {
    pub x: f64,
    pub y: f64,
    pub id: i64,
    pub time_offset: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseInteractionData {
    pub source: IncrementalSource,
    #[serde(rename = "type")]
    pub kind: MouseInteractionKind,
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer_type: Option<PointerType>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScrollData {
    pub source: IncrementalSource,
    pub id: i64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewportResizeData {
    pub source: IncrementalSource,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputData<'a> {
    pub source: IncrementalSource,
    pub id: i64,
    #[serde(borrow)]
    pub text: Cow<'a, str>,
    pub is_checked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_triggered: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInteractionData {
    pub source: IncrementalSource,
    #[serde(rename = "type")]
    pub kind: MediaInteractionKind,
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_time: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(default, rename = "loop", skip_serializing_if = "Option::is_none")]
    pub loop_: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_rate: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSheetRuleData<'a> {
    pub source: IncrementalSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<i64>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub adds: Option<Vec<StyleSheetAddRule<'a>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removes: Option<Vec<StyleSheetDeleteRule>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub replace_sync: Option<Cow<'a, str>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSheetRuleDataCompressed<'a> {
    pub source: IncrementalSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<i64>,
    #[serde(borrow, default)]
    pub adds: Cow<'a, str>,
    #[serde(borrow, default)]
    pub removes: Cow<'a, str>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub replace_sync: Option<Cow<'a, str>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StyleSheetAddRule<'a> {
    #[serde(borrow)]
    pub rule: Cow<'a, str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<StyleSheetIndex>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StyleSheetDeleteRule {
    pub index: StyleSheetIndex,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StyleSheetIndex {
    Single(u32),
    Nested(Vec<u32>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleDeclarationData<'a> {
    pub source: IncrementalSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<i64>,
    pub index: Vec<u32>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub set: Option<StyleDeclarationSet<'a>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<StyleDeclarationRemove<'a>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StyleDeclarationSet<'a> {
    #[serde(borrow)]
    pub property: Cow<'a, str>,
    #[serde(borrow)]
    pub value: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Cow<'a, str>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StyleDeclarationRemove<'a> {
    #[serde(borrow)]
    pub property: Cow<'a, str>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasMutationData<'a> {
    pub source: IncrementalSource,
    pub id: i64,
    #[serde(rename = "type")]
    pub ctx: CanvasContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_height: Option<u32>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<CanvasMutationCommand<'a>>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub property: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<simd_json::OwnedValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setter: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CanvasMutationCommand<'a> {
    #[serde(borrow)]
    pub property: Cow<'a, str>,
    pub args: simd_json::OwnedValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setter: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontData<'a> {
    pub source: IncrementalSource,
    #[serde(borrow)]
    pub family: Cow<'a, str>,
    #[serde(borrow)]
    pub font_source: Cow<'a, str>,
    pub buffer: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descriptors: Option<simd_json::OwnedValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SelectionData {
    pub source: IncrementalSource,
    pub ranges: Vec<SelectionRange>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRange {
    pub start: i64,
    pub start_offset: i64,
    pub end: i64,
    pub end_offset: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedStyleSheetData<'a> {
    pub source: IncrementalSource,
    pub id: i64,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<AdoptedStyleSheetStyle<'a>>>,
    pub style_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedStyleSheetStyle<'a> {
    pub style_id: i64,
    #[serde(borrow)]
    pub rules: Vec<StyleSheetAddRule<'a>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomElementData<'a> {
    pub source: IncrementalSource,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub define: Option<CustomElementDefine<'a>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CustomElementDefine<'a> {
    #[serde(borrow)]
    pub name: Cow<'a, str>,
}

#[derive(Debug)]
pub struct WindowedEvent<'a, E> {
    pub window_id: Cow<'a, str>,
    pub event: E,
}

impl<'de: 'a, 'a, E> Deserialize<'de> for WindowedEvent<'a, E>
where
    E: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (window_id, event) = <(Cow<'a, str>, E)>::deserialize(d)?;
        Ok(Self { window_id, event })
    }
}

impl<'a, E: Serialize> Serialize for WindowedEvent<'a, E> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (&self.window_id, &self.event).serialize(s)
    }
}

use anyhow::{Context, Result, bail};
use std::io::Read;

#[derive(Debug, Default)]
pub struct EventScan {
    pub ty: Option<u8>,
    pub source: Option<u8>,
    pub compressed: bool,
    pub data_range: Option<(usize, usize)>,
}

// `event_depth` = depth at which the event object opens. Bare events open
// it at depth 1; PostHog tuple form `[window_id, event]` opens it at depth 2.
// Only matches keys *at* event_depth — nested `"type":` inside `data`
// (e.g. MouseInteractionKind) is correctly ignored.
pub fn scan_event(line: &[u8]) -> EventScan {
    let mut out = EventScan::default();
    let mut pos = 0usize;
    let mut depth = 0u32;
    let mut event_depth: Option<u32> = None;

    while pos < line.len() {
        let b = line[pos];
        match b {
            b'"' => {
                let rest = &line[pos..];
                // `type`, `cv`, `data` are event-object keys (event_depth);
                // `source` lives one level deeper inside `data`. First-match
                // wins handles both cases.
                if event_depth == Some(depth) {
                    if out.ty.is_none() && rest.starts_with(b"\"type\":") {
                        out.ty = read_uint(line, pos + b"\"type\":".len());
                    } else if !out.compressed && rest.starts_with(b"\"cv\":") {
                        out.compressed = true;
                    } else if out.data_range.is_none() && rest.starts_with(b"\"data\":") {
                        out.data_range = locate_value(line, pos + b"\"data\":".len());
                    }
                }
                if out.source.is_none() && rest.starts_with(b"\"source\":") {
                    out.source = read_uint(line, pos + b"\"source\":".len());
                }
                pos = match skip_string(line, pos) {
                    Some(p) => p,
                    None => return out,
                };
            }
            b'{' => {
                depth += 1;
                if event_depth.is_none() {
                    event_depth = Some(depth);
                }
                pos += 1;
            }
            b'[' => {
                depth += 1;
                pos += 1;
            }
            b'}' | b']' => {
                match depth.checked_sub(1) {
                    Some(d) => depth = d,
                    None => return out,
                }
                pos += 1;
            }
            _ => pos += 1,
        }
    }
    out
}

pub fn data_value_range(line: &[u8]) -> Option<(usize, usize)> {
    scan_event(line).data_range
}

fn read_uint(line: &[u8], mut pos: usize) -> Option<u8> {
    while line.get(pos) == Some(&b' ') {
        pos += 1;
    }
    let mut n: u32 = 0;
    let mut saw = false;
    while let Some(&b) = line.get(pos) {
        if b.is_ascii_digit() {
            n = n * 10 + u32::from(b - b'0');
            saw = true;
            pos += 1;
        } else {
            break;
        }
    }
    saw.then_some(n as u8)
}

fn locate_value(line: &[u8], mut start: usize) -> Option<(usize, usize)> {
    while start < line.len() && line[start].is_ascii_whitespace() {
        start += 1;
    }
    let end = match *line.get(start)? {
        b'"' => skip_string(line, start)?,
        b'{' => skip_balanced(line, start, b'{', b'}')?,
        b'[' => skip_balanced(line, start, b'[', b']')?,
        _ => skip_scalar(line, start),
    };
    Some((start, end))
}

fn skip_string(line: &[u8], start: usize) -> Option<usize> {
    debug_assert_eq!(line[start], b'"');
    let mut pos = start + 1;
    while pos < line.len() {
        match line[pos] {
            b'\\' if pos + 1 < line.len() => pos += 2,
            b'"' => return Some(pos + 1),
            _ => pos += 1,
        }
    }
    None
}

fn skip_balanced(line: &[u8], start: usize, open: u8, close: u8) -> Option<usize> {
    debug_assert_eq!(line[start], open);
    let mut depth: u32 = 0;
    let mut pos = start;
    while pos < line.len() {
        let b = line[pos];
        if b == b'"' {
            pos = skip_string(line, pos)?;
            continue;
        }
        if b == open {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(pos + 1);
            }
        }
        pos += 1;
    }
    None
}

fn skip_scalar(line: &[u8], start: usize) -> usize {
    let mut pos = start;
    while pos < line.len() {
        let b = line[pos];
        if matches!(b, b',' | b'}' | b']') || b.is_ascii_whitespace() {
            return pos;
        }
        pos += 1;
    }
    pos
}

// Only handles the whole-blob (FullSnapshot-style) compression. Sub-field
// compression (Mutation, StyleSheetRule) is the caller's job.
pub fn extract_payload(
    line: &[u8],
    compressed: bool,
    scratch: &mut Vec<u8>,
) -> Result<(usize, usize)> {
    let (start, end) = data_value_range(line).context("no top-level `data` field")?;
    scratch.clear();
    if compressed && line.get(start) == Some(&b'"') {
        decompress_string_into(&line[start..end], scratch)?;
    } else {
        scratch.extend_from_slice(&line[start..end]);
    }
    Ok((start, end))
}

// PostHog wire format: each gzip byte stored as its U+00XX codepoint
// (latin-1), then JSON-string-escaped.
pub fn decompress_string_into(quoted: &[u8], dst: &mut Vec<u8>) -> Result<()> {
    if quoted.len() < 2 || quoted[0] != b'"' || quoted[quoted.len() - 1] != b'"' {
        bail!("compressed `data` is not a JSON string");
    }
    // simd-json needs `&mut [u8]`; the input slice is shared.
    let mut owned = quoted.to_vec();
    let raw: Vec<u8> = {
        let s: std::borrow::Cow<'_, str> =
            simd_json::serde::from_slice(&mut owned).context("parse cv data string")?;
        let mut out = Vec::with_capacity(s.len());
        for c in s.chars() {
            let cp = c as u32;
            if cp > 0xFF {
                bail!("codepoint U+{cp:04X} > 0xFF in latin-1 gzip stream");
            }
            out.push(cp as u8);
        }
        out
    };
    let mut gz = flate2::read::GzDecoder::new(&raw[..]);
    gz.read_to_end(dst).context("gunzip cv data")?;
    Ok(())
}

pub fn emit_with_payload(
    line: &[u8],
    data_range: (usize, usize),
    payload: &[u8],
    compressed: bool,
    out: &mut Vec<u8>,
) -> Result<()> {
    out.extend_from_slice(&line[..data_range.0]);
    if compressed {
        write_compressed_string(payload, out)?;
    } else {
        out.extend_from_slice(payload);
    }
    out.extend_from_slice(&line[data_range.1..]);
    Ok(())
}

fn write_compressed_string(payload: &[u8], out: &mut Vec<u8>) -> Result<()> {
    use std::io::Write;
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(payload).context("gzip payload")?;
    let zipped = gz.finish().context("finish gzip")?;
    out.push(b'"');
    for &b in &zipped {
        match b {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0c => out.extend_from_slice(b"\\f"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x00..=0x1f | 0x7f..=0xff => {
                let _ = write!(out, "\\u{:04x}", b);
            }
            _ => out.push(b),
        }
    }
    out.push(b'"');
    Ok(())
}

pub fn decompress_subfield_into(s: &str, dst: &mut Vec<u8>) -> Result<()> {
    let mut raw: Vec<u8> = Vec::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if cp > 0xFF {
            bail!("codepoint U+{cp:04X} > 0xFF in latin-1 gzip stream");
        }
        raw.push(cp as u8);
    }
    let mut gz = flate2::read::GzDecoder::new(&raw[..]);
    gz.read_to_end(dst).context("gunzip sub-field")?;
    Ok(())
}

pub fn compress_subfield_to_string(payload: &[u8]) -> Result<String> {
    use std::io::Write;
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(payload).context("gzip sub-field")?;
    let zipped = gz.finish().context("finish gzip sub-field")?;
    let mut s = String::with_capacity(zipped.len());
    for b in zipped {
        s.push(b as char);
    }
    Ok(s)
}

#[derive(Default)]
pub struct MutationSubScratch {
    pub texts: Vec<u8>,
    pub attributes: Vec<u8>,
    pub removes: Vec<u8>,
    pub adds: Vec<u8>,
}

impl<'a> MetaData<'a> {
    pub fn read(line: &[u8], scratch: &'a mut Vec<u8>) -> Result<((usize, usize), Self)> {
        let range = extract_payload(line, false, scratch)?;
        let data = simd_json::serde::from_slice(scratch.as_mut_slice())?;
        Ok((range, data))
    }

    pub fn write(
        &self,
        line: &[u8],
        range: (usize, usize),
        payload_buf: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        payload_buf.clear();
        simd_json::serde::to_writer(&mut *payload_buf, self)?;
        emit_with_payload(line, range, payload_buf, false, out)
    }
}

impl<'a> InputData<'a> {
    pub fn read(line: &[u8], scratch: &'a mut Vec<u8>) -> Result<((usize, usize), Self)> {
        let range = extract_payload(line, false, scratch)?;
        let data = simd_json::serde::from_slice(scratch.as_mut_slice())?;
        Ok((range, data))
    }

    pub fn write(
        &self,
        line: &[u8],
        range: (usize, usize),
        payload_buf: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        payload_buf.clear();
        simd_json::serde::to_writer(&mut *payload_buf, self)?;
        emit_with_payload(line, range, payload_buf, false, out)
    }
}

impl<'a> CustomEventData<'a> {
    pub fn read(line: &[u8], scratch: &'a mut Vec<u8>) -> Result<((usize, usize), Self)> {
        let range = extract_payload(line, false, scratch)?;
        let data = simd_json::serde::from_slice(scratch.as_mut_slice())?;
        Ok((range, data))
    }

    pub fn write(
        &self,
        line: &[u8],
        range: (usize, usize),
        payload_buf: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        payload_buf.clear();
        simd_json::serde::to_writer(&mut *payload_buf, self)?;
        emit_with_payload(line, range, payload_buf, false, out)
    }
}

impl<'a> PluginEventData<'a> {
    pub fn read(line: &[u8], scratch: &'a mut Vec<u8>) -> Result<((usize, usize), Self)> {
        let range = extract_payload(line, false, scratch)?;
        let data = simd_json::serde::from_slice(scratch.as_mut_slice())?;
        Ok((range, data))
    }

    pub fn write(
        &self,
        line: &[u8],
        range: (usize, usize),
        payload_buf: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        payload_buf.clear();
        simd_json::serde::to_writer(&mut *payload_buf, self)?;
        emit_with_payload(line, range, payload_buf, false, out)
    }
}

impl<'a> FullSnapshotData<'a> {
    pub fn read(
        line: &[u8],
        compressed: bool,
        scratch: &'a mut Vec<u8>,
    ) -> Result<((usize, usize), Self)> {
        let range = extract_payload(line, compressed, scratch)?;
        let data = simd_json::serde::from_slice(scratch.as_mut_slice())?;
        Ok((range, data))
    }

    pub fn write(
        &self,
        line: &[u8],
        range: (usize, usize),
        compressed: bool,
        payload_buf: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        payload_buf.clear();
        simd_json::serde::to_writer(&mut *payload_buf, self)?;
        emit_with_payload(line, range, payload_buf, compressed, out)
    }
}

impl<'a> MutationData<'a> {
    pub fn read(
        line: &[u8],
        compressed: bool,
        data_buf: &'a mut Vec<u8>,
        sub: &'a mut MutationSubScratch,
    ) -> Result<((usize, usize), Self)> {
        // Mutation's compressed form compresses sub-fields, not the whole data blob.
        let range = extract_payload(line, false, data_buf)?;

        if !compressed {
            let data = simd_json::serde::from_slice(data_buf.as_mut_slice())?;
            return Ok((range, data));
        }

        let (source, is_attach_iframe) = {
            let cd: MutationDataCompressed<'_> =
                simd_json::serde::from_slice(data_buf.as_mut_slice())?;
            sub.texts.clear();
            decompress_subfield_into(&cd.texts, &mut sub.texts)?;
            sub.attributes.clear();
            decompress_subfield_into(&cd.attributes, &mut sub.attributes)?;
            sub.removes.clear();
            decompress_subfield_into(&cd.removes, &mut sub.removes)?;
            sub.adds.clear();
            decompress_subfield_into(&cd.adds, &mut sub.adds)?;
            (cd.source, cd.is_attach_iframe)
        };

        let texts: Vec<TextMutation<'a>> = simd_json::serde::from_slice(sub.texts.as_mut_slice())?;
        let attributes: Vec<AttributeMutation<'a>> =
            simd_json::serde::from_slice(sub.attributes.as_mut_slice())?;
        let removes: Vec<RemovedNodeMutation> =
            simd_json::serde::from_slice(sub.removes.as_mut_slice())?;
        let adds: Vec<AddedNodeMutation<'a>> =
            simd_json::serde::from_slice(sub.adds.as_mut_slice())?;

        Ok((
            range,
            MutationData {
                source,
                texts,
                attributes,
                removes,
                adds,
                is_attach_iframe,
            },
        ))
    }

    pub fn write(
        &self,
        line: &[u8],
        range: (usize, usize),
        compressed: bool,
        payload: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        payload.clear();

        if !compressed {
            simd_json::serde::to_writer(&mut *payload, self)?;
        } else {
            let mut buf = Vec::new();
            buf.clear();
            simd_json::serde::to_writer(&mut buf, &self.texts)?;
            let texts = compress_subfield_to_string(&buf)?;
            buf.clear();
            simd_json::serde::to_writer(&mut buf, &self.attributes)?;
            let attributes = compress_subfield_to_string(&buf)?;
            buf.clear();
            simd_json::serde::to_writer(&mut buf, &self.removes)?;
            let removes = compress_subfield_to_string(&buf)?;
            buf.clear();
            simd_json::serde::to_writer(&mut buf, &self.adds)?;
            let adds = compress_subfield_to_string(&buf)?;

            let cd = MutationDataCompressed {
                source: self.source,
                texts: std::borrow::Cow::Owned(texts),
                attributes: std::borrow::Cow::Owned(attributes),
                removes: std::borrow::Cow::Owned(removes),
                adds: std::borrow::Cow::Owned(adds),
                is_attach_iframe: self.is_attach_iframe,
            };
            simd_json::serde::to_writer(&mut *payload, &cd)?;
        }

        emit_with_payload(line, range, payload, false, out)
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
