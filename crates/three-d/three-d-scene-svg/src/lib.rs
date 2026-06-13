#![doc = include_str!("../README.md")]

pub mod surface;
use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use three_d_processing_core::{Point3, Quaternion, TrsTransform3, Vector3};
use three_d_processing_mesh::{Mesh, Triangle};
use video_analysis_core::{DetectError, Result};

/// Scene Vector 3D schema identifier.
pub const SCHEMA: &str = "scene-vector-3d";
/// Current Scene Vector 3D profile version.
pub const VERSION: &str = "0.1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for scene document.
pub struct SceneDocument {
    /// Schema identifier.
    pub schema: String,
    /// Profile version.
    pub version: String,
    /// Optional title.
    pub title: Option<String>,
    /// Preview viewport.
    pub viewport: SceneViewport,
    /// Projection camera.
    pub camera: Camera,
    /// Root node.
    pub root: Node,
}

impl SceneDocument {
    /// Creates a new scene document.
    pub fn new(viewport: SceneViewport, camera: Camera, root: Node) -> Result<Self> {
        let scene = Self {
            schema: SCHEMA.to_string(),
            version: VERSION.to_string(),
            title: None,
            viewport,
            camera,
            root,
        };
        scene.validate()?;
        Ok(scene)
    }

    /// Sets the document title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Validates this document.
    pub fn validate(&self) -> Result<()> {
        validate_document(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for scene viewport.
pub struct SceneViewport {
    /// Width in output pixels.
    pub width: u32,
    /// Height in output pixels.
    pub height: u32,
}

impl SceneViewport {
    /// Creates a new viewport.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let viewport = Self { width, height };
        viewport.validate()?;
        Ok(viewport)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument("viewport dimensions must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
/// Data type for camera.
pub enum Camera {
    /// Orthographic projection.
    Orthographic {
        /// Camera origin.
        eye: Point3,
        /// Look-at target.
        target: Point3,
        /// Up direction.
        up: Vector3,
        /// Vertical world-unit scale.
        scale: f32,
    },
    /// Perspective projection.
    Perspective {
        /// Camera origin.
        eye: Point3,
        /// Look-at target.
        target: Point3,
        /// Up direction.
        up: Vector3,
        /// Vertical field of view in degrees.
        fov_y_degrees: f32,
        /// Near clip distance.
        near: f32,
        /// Far clip distance.
        far: f32,
    },
}

impl Camera {
    /// Creates an orthographic camera with +Y up.
    pub fn orthographic(eye: Point3, target: Point3, scale: f32) -> Result<Self> {
        let camera = Self::Orthographic {
            eye,
            target,
            up: Vector3::new(0.0, 1.0, 0.0),
            scale,
        };
        camera.validate()?;
        Ok(camera)
    }

    /// Creates a perspective camera with +Y up.
    pub fn perspective(
        eye: Point3,
        target: Point3,
        fov_y_degrees: f32,
        near: f32,
        far: f32,
    ) -> Result<Self> {
        let camera = Self::Perspective {
            eye,
            target,
            up: Vector3::new(0.0, 1.0, 0.0),
            fov_y_degrees,
            near,
            far,
        };
        camera.validate()?;
        Ok(camera)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        match self {
            Self::Orthographic {
                eye,
                target,
                up,
                scale,
            } => {
                validate_points(&[eye, target])?;
                validate_vector(up, "camera up")?;
                validate_camera_basis(eye, target, up)?;
                validate_positive(scale, "orthographic scale")?;
            }
            Self::Perspective {
                eye,
                target,
                up,
                fov_y_degrees,
                near,
                far,
            } => {
                validate_points(&[eye, target])?;
                validate_vector(up, "camera up")?;
                validate_camera_basis(eye, target, up)?;
                validate_positive(fov_y_degrees, "perspective fov")?;
                if fov_y_degrees >= 180.0 {
                    return Err(invalid_argument("perspective fov must be less than 180"));
                }
                validate_positive(near, "near clip")?;
                validate_positive(far, "far clip")?;
                if near >= far {
                    return Err(invalid_argument("near clip must be less than far clip"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for node transform.
pub struct NodeTransform {
    /// Local translation.
    pub translation: Vector3,
    /// Local rotation.
    pub rotation: Quaternion,
    /// Local non-uniform scale.
    pub scale: Vector3,
}

impl Default for NodeTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl NodeTransform {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        rotation: Quaternion::IDENTITY,
        scale: Vector3::new(1.0, 1.0, 1.0),
    };

    /// Creates a new transform.
    pub fn new(translation: Vector3, rotation: Quaternion, scale: Vector3) -> Result<Self> {
        let transform = Self {
            translation,
            rotation: rotation.normalize()?,
            scale,
        };
        transform.validate()?;
        Ok(transform)
    }

    /// Creates a translation transform.
    pub fn translation(translation: Vector3) -> Result<Self> {
        Self::new(
            translation,
            Quaternion::IDENTITY,
            Vector3::new(1.0, 1.0, 1.0),
        )
    }

    /// Creates a uniform scaling transform.
    pub fn uniform_scale(scale: f32) -> Result<Self> {
        Self::new(
            Vector3::ZERO,
            Quaternion::IDENTITY,
            Vector3::new(scale, scale, scale),
        )
    }

    /// Applies the transform to a point.
    pub fn apply_point(self, point: Point3) -> Result<Point3> {
        self.to_core_trs()?.apply_point(point)
    }

    /// Composes this transform with a child transform.
    pub fn compose(self, child: Self) -> Result<Self> {
        self.validate()?;
        child.validate()?;
        let child_translation = self.apply_vector(child.translation)?;
        let rotation = self.rotation.mul_quaternion(child.rotation)?;
        Self::new(
            self.translation + child_translation,
            rotation,
            Vector3::new(
                self.scale.x * child.scale.x,
                self.scale.y * child.scale.y,
                self.scale.z * child.scale.z,
            ),
        )
    }

    fn apply_vector(self, vector: Vector3) -> Result<Vector3> {
        self.to_core_trs()?.apply_vector(vector)
    }

    /// Converts this scene transform to the canonical 3D core TRS type.
    pub fn to_core_trs(self) -> Result<TrsTransform3> {
        TrsTransform3::new(self.translation, self.rotation, self.scale)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        validate_vector(self.translation, "transform translation")?;
        validate_vector(self.scale, "transform scale")?;
        self.rotation.normalize()?;
        if self.scale.x == 0.0 || self.scale.y == 0.0 || self.scale.z == 0.0 {
            return Err(invalid_argument(
                "transform scale components must be non-zero",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for RGBA color.
pub struct ColorRgba {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl ColorRgba {
    /// Creates an opaque RGB color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates an RGBA color.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for inherited static paint style.
pub struct Style {
    /// Optional fill color.
    pub fill: Option<ColorRgba>,
    /// Optional stroke color.
    pub stroke: Option<ColorRgba>,
    /// Stroke width in output pixels.
    pub stroke_width: Option<f32>,
    /// Opacity multiplier.
    pub opacity: Option<f32>,
    /// Visibility flag.
    pub visible: Option<bool>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fill: Some(ColorRgba::rgb(220, 225, 232)),
            stroke: Some(ColorRgba::rgb(25, 25, 25)),
            stroke_width: Some(1.0),
            opacity: Some(1.0),
            visible: Some(true),
        }
    }
}

impl Style {
    /// Creates a stroke-only style.
    pub fn stroke(color: ColorRgba, width: f32) -> Self {
        Self {
            fill: None,
            stroke: Some(color),
            stroke_width: Some(width),
            opacity: Some(1.0),
            visible: Some(true),
        }
    }

    /// Creates a fill style.
    pub fn fill(color: ColorRgba) -> Self {
        Self {
            fill: Some(color),
            stroke: None,
            stroke_width: Some(1.0),
            opacity: Some(1.0),
            visible: Some(true),
        }
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if let Some(width) = self.stroke_width {
            validate_positive(width, "stroke width")?;
        }
        if let Some(opacity) = self.opacity {
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
                return Err(invalid_argument(
                    "opacity must be finite and between 0 and 1",
                ));
            }
        }
        Ok(())
    }

    fn resolve(self, parent: ResolvedStyle) -> Result<ResolvedStyle> {
        self.validate()?;
        Ok(ResolvedStyle {
            fill: self.fill.or(parent.fill),
            stroke: self.stroke.or(parent.stroke),
            stroke_width: self.stroke_width.unwrap_or(parent.stroke_width),
            opacity: self.opacity.unwrap_or(parent.opacity),
            visible: self.visible.unwrap_or(parent.visible),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResolvedStyle {
    fill: Option<ColorRgba>,
    stroke: Option<ColorRgba>,
    stroke_width: f32,
    opacity: f32,
    visible: bool,
}

impl Default for ResolvedStyle {
    fn default() -> Self {
        let style = Style::default();
        Self {
            fill: style.fill,
            stroke: style.stroke,
            stroke_width: style.stroke_width.unwrap_or(1.0),
            opacity: style.opacity.unwrap_or(1.0),
            visible: style.visible.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
/// Data type for scene node.
pub enum Node {
    /// Group node.
    Group {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Child nodes.
        children: Vec<Node>,
    },
    /// Point primitive.
    Point {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Point position.
        position: Point3,
        /// Preview radius in pixels.
        radius: f32,
    },
    /// Line primitive.
    Line {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Start point.
        from: Point3,
        /// End point.
        to: Point3,
    },
    /// Polyline primitive.
    Polyline {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Ordered points.
        points: Vec<Point3>,
        /// Whether the shape is closed.
        closed: bool,
    },
    /// Triangle mesh primitive.
    Mesh {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Mesh vertices.
        vertices: Vec<Point3>,
        /// Mesh triangles.
        triangles: Vec<Triangle>,
    },
    /// Axis-aligned box primitive.
    Box {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Box center.
        center: Point3,
        /// Box size.
        size: Vector3,
    },
    /// Sphere primitive.
    Sphere {
        /// Optional node identifier.
        id: Option<String>,
        /// Local transform.
        transform: NodeTransform,
        /// Local style overrides.
        style: Option<Style>,
        /// Sphere center.
        center: Point3,
        /// Sphere radius.
        radius: f32,
    },
}

impl Node {
    /// Creates a group node.
    pub fn group(children: impl Into<Vec<Node>>) -> Self {
        Self::Group {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            children: children.into(),
        }
    }

    /// Creates a point node.
    pub fn point(position: Point3) -> Self {
        Self::Point {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            position,
            radius: 3.0,
        }
    }

    /// Creates a line node.
    pub fn line(from: Point3, to: Point3) -> Self {
        Self::Line {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            from,
            to,
        }
    }

    /// Creates a polyline node.
    pub fn polyline(points: impl Into<Vec<Point3>>) -> Self {
        Self::Polyline {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            points: points.into(),
            closed: false,
        }
    }

    /// Creates a mesh node.
    pub fn mesh(mesh: Mesh) -> Self {
        Self::Mesh {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            vertices: mesh.vertices,
            triangles: mesh.triangles,
        }
    }

    /// Creates a box node.
    pub fn cuboid(center: Point3, size: Vector3) -> Self {
        Self::Box {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            center,
            size,
        }
    }

    /// Creates a sphere node.
    pub fn sphere(center: Point3, radius: f32) -> Self {
        Self::Sphere {
            id: None,
            transform: NodeTransform::IDENTITY,
            style: None,
            center,
            radius,
        }
    }

    /// Sets the node id.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        *self.id_mut() = Some(id.into());
        self
    }

    /// Sets the node transform.
    pub fn transform(mut self, transform: NodeTransform) -> Self {
        *self.transform_mut() = transform;
        self
    }

    /// Sets the node style.
    pub fn style(mut self, style: Style) -> Self {
        *self.style_mut() = Some(style);
        self
    }

    /// Makes a polyline closed. Other node kinds are unchanged.
    pub fn closed(mut self) -> Self {
        if let Self::Polyline { closed, .. } = &mut self {
            *closed = true;
        }
        self
    }

    /// Validates this node tree.
    pub fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        validate_node(self, &mut ids)
    }

    fn id_ref(&self) -> Option<&str> {
        match self {
            Self::Group { id, .. }
            | Self::Point { id, .. }
            | Self::Line { id, .. }
            | Self::Polyline { id, .. }
            | Self::Mesh { id, .. }
            | Self::Box { id, .. }
            | Self::Sphere { id, .. } => id.as_deref(),
        }
    }

    fn id_mut(&mut self) -> &mut Option<String> {
        match self {
            Self::Group { id, .. }
            | Self::Point { id, .. }
            | Self::Line { id, .. }
            | Self::Polyline { id, .. }
            | Self::Mesh { id, .. }
            | Self::Box { id, .. }
            | Self::Sphere { id, .. } => id,
        }
    }

    fn local_transform(&self) -> NodeTransform {
        match self {
            Self::Group { transform, .. }
            | Self::Point { transform, .. }
            | Self::Line { transform, .. }
            | Self::Polyline { transform, .. }
            | Self::Mesh { transform, .. }
            | Self::Box { transform, .. }
            | Self::Sphere { transform, .. } => *transform,
        }
    }

    fn transform_mut(&mut self) -> &mut NodeTransform {
        match self {
            Self::Group { transform, .. }
            | Self::Point { transform, .. }
            | Self::Line { transform, .. }
            | Self::Polyline { transform, .. }
            | Self::Mesh { transform, .. }
            | Self::Box { transform, .. }
            | Self::Sphere { transform, .. } => transform,
        }
    }

    fn local_style(&self) -> Option<Style> {
        match self {
            Self::Group { style, .. }
            | Self::Point { style, .. }
            | Self::Line { style, .. }
            | Self::Polyline { style, .. }
            | Self::Mesh { style, .. }
            | Self::Box { style, .. }
            | Self::Sphere { style, .. } => *style,
        }
    }

    fn style_mut(&mut self) -> &mut Option<Style> {
        match self {
            Self::Group { style, .. }
            | Self::Point { style, .. }
            | Self::Line { style, .. }
            | Self::Polyline { style, .. }
            | Self::Mesh { style, .. }
            | Self::Box { style, .. }
            | Self::Sphere { style, .. } => style,
        }
    }
}

/// Validates a Scene Vector 3D document.
pub fn validate_document(scene: &SceneDocument) -> Result<()> {
    if scene.schema != SCHEMA {
        return Err(invalid_argument(format!(
            "schema must be `{SCHEMA}`, got `{}`",
            scene.schema
        )));
    }
    if scene.version != VERSION {
        return Err(invalid_argument(format!(
            "version must be `{VERSION}`, got `{}`",
            scene.version
        )));
    }
    scene.viewport.validate()?;
    scene.camera.validate()?;
    scene.root.validate()
}

/// Serializes a document as pretty JSON.
pub fn to_json_pretty(scene: &SceneDocument) -> Result<String> {
    scene.validate()?;
    serde_json::to_string_pretty(scene)
        .map_err(|err| invalid_argument(format!("failed to serialize scene: {err}")))
}

/// Parses and validates a document from JSON.
pub fn from_json_str(input: &str) -> Result<SceneDocument> {
    let scene = serde_json::from_str::<SceneDocument>(input)
        .map_err(|err| invalid_argument(format!("failed to parse scene JSON: {err}")))?;
    scene.validate()?;
    Ok(scene)
}

/// Renders a deterministic SVG preview.
pub fn render_svg(scene: &SceneDocument) -> Result<String> {
    scene.validate()?;
    let projector = Projector::new(scene.camera, scene.viewport)?;
    let mut items = Vec::new();
    collect_render_items(
        &scene.root,
        NodeTransform::IDENTITY,
        ResolvedStyle::default(),
        &projector,
        &mut items,
    )?;
    items.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.order.cmp(&b.order))
    });

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img">"#,
        scene.viewport.width, scene.viewport.height, scene.viewport.width, scene.viewport.height
    ));
    if let Some(title) = &scene.title {
        svg.push_str("<title>");
        svg.push_str(&escape_xml(title));
        svg.push_str("</title>");
    }
    for item in items {
        svg.push_str(&item.svg);
    }
    svg.push_str("</svg>");
    Ok(svg)
}

#[derive(Debug, Clone)]
struct RenderItem {
    order: usize,
    depth: f32,
    svg: String,
}

#[derive(Debug, Clone, Copy)]
struct Projector {
    camera: Camera,
    viewport: SceneViewport,
    right: Vector3,
    up: Vector3,
    forward: Vector3,
}

impl Projector {
    fn new(camera: Camera, viewport: SceneViewport) -> Result<Self> {
        let (eye, target, up) = camera_vectors(camera);
        let forward = (target - eye).normalize()?;
        let right = forward.cross(up).normalize()?;
        let up = right.cross(forward).normalize()?;
        Ok(Self {
            camera,
            viewport,
            right,
            up,
            forward,
        })
    }

    fn project(self, point: Point3) -> Option<ProjectedPoint> {
        let (eye, _, _) = camera_vectors(self.camera);
        let relative = point - eye;
        let x = relative.dot(self.right);
        let y = relative.dot(self.up);
        let depth = relative.dot(self.forward);
        match self.camera {
            Camera::Orthographic { scale, .. } => {
                let pixels_per_unit = self.viewport.height as f32 / scale;
                Some(ProjectedPoint {
                    x: self.viewport.width as f32 * 0.5 + x * pixels_per_unit,
                    y: self.viewport.height as f32 * 0.5 - y * pixels_per_unit,
                    depth,
                })
            }
            Camera::Perspective {
                fov_y_degrees,
                near,
                far,
                ..
            } => {
                if depth <= near || depth >= far {
                    return None;
                }
                let focal =
                    self.viewport.height as f32 / (2.0 * (fov_y_degrees.to_radians() * 0.5).tan());
                Some(ProjectedPoint {
                    x: self.viewport.width as f32 * 0.5 + (x * focal / depth),
                    y: self.viewport.height as f32 * 0.5 - (y * focal / depth),
                    depth,
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ProjectedPoint {
    x: f32,
    y: f32,
    depth: f32,
}

fn collect_render_items(
    node: &Node,
    parent_transform: NodeTransform,
    parent_style: ResolvedStyle,
    projector: &Projector,
    items: &mut Vec<RenderItem>,
) -> Result<()> {
    let transform = parent_transform.compose(node.local_transform())?;
    let style = if let Some(style) = node.local_style() {
        style.resolve(parent_style)?
    } else {
        parent_style
    };
    if !style.visible {
        return Ok(());
    }

    match node {
        Node::Group { children, .. } => {
            for child in children {
                collect_render_items(child, transform, style, projector, items)?;
            }
        }
        Node::Point {
            position, radius, ..
        } => {
            validate_positive(*radius, "point radius")?;
            let point = transform.apply_point(*position)?;
            if let Some(projected) = projector.project(point) {
                let svg = format!(
                    r#"<circle cx="{}" cy="{}" r="{}" {}/>"#,
                    fmt(projected.x),
                    fmt(projected.y),
                    fmt(*radius),
                    svg_style(style, true)
                );
                push_item(items, projected.depth, svg);
            }
        }
        Node::Line { from, to, .. } => {
            let from = transform.apply_point(*from)?;
            let to = transform.apply_point(*to)?;
            if let (Some(a), Some(b)) = (projector.project(from), projector.project(to)) {
                let svg = format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {}/>"#,
                    fmt(a.x),
                    fmt(a.y),
                    fmt(b.x),
                    fmt(b.y),
                    svg_style(style, false)
                );
                push_item(items, (a.depth + b.depth) * 0.5, svg);
            }
        }
        Node::Polyline { points, closed, .. } => {
            let projected = points
                .iter()
                .copied()
                .map(|point| {
                    transform
                        .apply_point(point)
                        .ok()
                        .and_then(|point| projector.project(point))
                })
                .collect::<Option<Vec<_>>>();
            if let Some(projected) = projected {
                if projected.len() >= 2 {
                    let coordinates = projected
                        .iter()
                        .map(|point| format!("{},{}", fmt(point.x), fmt(point.y)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let depth = mean_depth(&projected);
                    let tag = if *closed { "polygon" } else { "polyline" };
                    let svg = format!(
                        r#"<{} points="{}" {}/>"#,
                        tag,
                        coordinates,
                        svg_style(style, *closed)
                    );
                    push_item(items, depth, svg);
                }
            }
        }
        Node::Mesh {
            vertices,
            triangles,
            ..
        } => {
            let mesh = Mesh::new(vertices.clone(), triangles.clone())?;
            for triangle in mesh.triangles {
                let projected = triangle
                    .vertices
                    .iter()
                    .map(|index| {
                        transform
                            .apply_point(mesh.vertices[*index])
                            .ok()
                            .and_then(|point| projector.project(point))
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(projected) = projected {
                    let coordinates = projected
                        .iter()
                        .map(|point| format!("{},{}", fmt(point.x), fmt(point.y)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let svg = format!(
                        r#"<polygon points="{}" {}/>"#,
                        coordinates,
                        svg_style(style, true)
                    );
                    push_item(items, mean_depth(&projected), svg);
                }
            }
        }
        Node::Box { center, size, .. } => {
            validate_positive(size.x.abs(), "box width")?;
            validate_positive(size.y.abs(), "box height")?;
            validate_positive(size.z.abs(), "box depth")?;
            let mesh = cuboid_mesh(*center, *size)?;
            let mesh_node = Node::mesh(mesh).style(Style {
                fill: style.fill,
                stroke: style.stroke,
                stroke_width: Some(style.stroke_width),
                opacity: Some(style.opacity),
                visible: Some(style.visible),
            });
            collect_render_items(
                &mesh_node,
                transform,
                ResolvedStyle::default(),
                projector,
                items,
            )?;
        }
        Node::Sphere { center, radius, .. } => {
            validate_positive(*radius, "sphere radius")?;
            let center = transform.apply_point(*center)?;
            if let Some(projected) = projector.project(center) {
                let scaled_radius = radius * transform.scale.x.abs().max(transform.scale.y.abs());
                let preview_radius = match projector.camera {
                    Camera::Orthographic { scale, .. } => {
                        scaled_radius * projector.viewport.height as f32 / scale
                    }
                    Camera::Perspective { fov_y_degrees, .. } => {
                        let focal = projector.viewport.height as f32
                            / (2.0 * (fov_y_degrees.to_radians() * 0.5).tan());
                        scaled_radius * focal / projected.depth.max(f32::EPSILON)
                    }
                };
                let svg = format!(
                    r#"<circle cx="{}" cy="{}" r="{}" {}/>"#,
                    fmt(projected.x),
                    fmt(projected.y),
                    fmt(preview_radius),
                    svg_style(style, true)
                );
                push_item(items, projected.depth, svg);
            }
        }
    }
    Ok(())
}

fn validate_node(node: &Node, ids: &mut BTreeSet<String>) -> Result<()> {
    if let Some(id) = node.id_ref() {
        if id.trim().is_empty() {
            return Err(invalid_argument("node id must not be empty"));
        }
        if !ids.insert(id.to_string()) {
            return Err(invalid_argument(format!("duplicate node id `{id}`")));
        }
    }
    node.local_transform().validate()?;
    if let Some(style) = node.local_style() {
        style.validate()?;
    }

    match node {
        Node::Group { children, .. } => {
            for child in children {
                validate_node(child, ids)?;
            }
        }
        Node::Point {
            position, radius, ..
        } => {
            validate_points(&[*position])?;
            validate_positive(*radius, "point radius")?;
        }
        Node::Line { from, to, .. } => {
            validate_points(&[*from, *to])?;
            if from == to {
                return Err(invalid_argument("line endpoints must be distinct"));
            }
        }
        Node::Polyline { points, .. } => {
            if points.len() < 2 {
                return Err(invalid_argument(
                    "polyline must contain at least two points",
                ));
            }
            validate_points(points)?;
        }
        Node::Mesh {
            vertices,
            triangles,
            ..
        } => {
            Mesh::new(vertices.clone(), triangles.clone())?;
        }
        Node::Box { center, size, .. } => {
            validate_points(&[*center])?;
            validate_vector(*size, "box size")?;
            if size.x == 0.0 || size.y == 0.0 || size.z == 0.0 {
                return Err(invalid_argument("box size components must be non-zero"));
            }
        }
        Node::Sphere { center, radius, .. } => {
            validate_points(&[*center])?;
            validate_positive(*radius, "sphere radius")?;
        }
    }
    Ok(())
}

fn camera_vectors(camera: Camera) -> (Point3, Point3, Vector3) {
    match camera {
        Camera::Orthographic {
            eye, target, up, ..
        }
        | Camera::Perspective {
            eye, target, up, ..
        } => (eye, target, up),
    }
}

fn validate_camera_basis(eye: Point3, target: Point3, up: Vector3) -> Result<()> {
    let forward = target - eye;
    if forward.length() <= f32::EPSILON {
        return Err(invalid_argument("camera eye and target must be distinct"));
    }
    if up.length() <= f32::EPSILON {
        return Err(invalid_argument("camera up vector must be non-zero"));
    }
    if forward.cross(up).length() <= f32::EPSILON {
        return Err(invalid_argument(
            "camera up vector must not be parallel to the view direction",
        ));
    }
    Ok(())
}

fn cuboid_mesh(center: Point3, size: Vector3) -> Result<Mesh> {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;
    let vertices = vec![
        Point3::new(center.x - hx, center.y - hy, center.z - hz),
        Point3::new(center.x + hx, center.y - hy, center.z - hz),
        Point3::new(center.x + hx, center.y + hy, center.z - hz),
        Point3::new(center.x - hx, center.y + hy, center.z - hz),
        Point3::new(center.x - hx, center.y - hy, center.z + hz),
        Point3::new(center.x + hx, center.y - hy, center.z + hz),
        Point3::new(center.x + hx, center.y + hy, center.z + hz),
        Point3::new(center.x - hx, center.y + hy, center.z + hz),
    ];
    let triangles = vec![
        Triangle::new(0, 1, 2),
        Triangle::new(0, 2, 3),
        Triangle::new(4, 6, 5),
        Triangle::new(4, 7, 6),
        Triangle::new(0, 4, 5),
        Triangle::new(0, 5, 1),
        Triangle::new(1, 5, 6),
        Triangle::new(1, 6, 2),
        Triangle::new(2, 6, 7),
        Triangle::new(2, 7, 3),
        Triangle::new(3, 7, 4),
        Triangle::new(3, 4, 0),
    ];
    Mesh::new(vertices, triangles)
}

fn svg_style(style: ResolvedStyle, allow_fill: bool) -> String {
    let fill = if allow_fill { style.fill } else { None };
    let fill_attr = match fill {
        Some(color) => format!(
            r#"fill="{}" fill-opacity="{}""#,
            color_hex(color),
            alpha(color)
        ),
        None => r#"fill="none""#.to_string(),
    };
    let stroke_attr = match style.stroke {
        Some(color) => format!(
            r#"stroke="{}" stroke-opacity="{}" stroke-width="{}""#,
            color_hex(color),
            alpha(color),
            fmt(style.stroke_width)
        ),
        None => r#"stroke="none""#.to_string(),
    };
    format!(
        r#"{fill_attr} {stroke_attr} opacity="{}" vector-effect="non-scaling-stroke""#,
        fmt(style.opacity)
    )
}

fn color_hex(color: ColorRgba) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

fn alpha(color: ColorRgba) -> String {
    fmt(color.a as f32 / 255.0)
}

fn push_item(items: &mut Vec<RenderItem>, depth: f32, svg: String) {
    items.push(RenderItem {
        order: items.len(),
        depth,
        svg,
    });
}

fn mean_depth(points: &[ProjectedPoint]) -> f32 {
    points.iter().map(|point| point.depth).sum::<f32>() / points.len() as f32
}

fn fmt(value: f32) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn validate_points(points: &[Point3]) -> Result<()> {
    for point in points {
        if !point.is_finite() {
            return Err(invalid_argument("points must be finite"));
        }
    }
    Ok(())
}

fn validate_vector(vector: Vector3, name: &str) -> Result<()> {
    if !vector.is_finite() {
        return Err(invalid_argument(format!("{name} must be finite")));
    }
    Ok(())
}

fn validate_positive(value: f32, name: &str) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(invalid_argument(format!(
            "{name} must be positive and finite"
        )))
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_round_trips_through_json() {
        let scene = sample_scene().unwrap().title("axes");
        let json = to_json_pretty(&scene).unwrap();
        assert!(json.contains("scene-vector-3d"));
        let parsed = from_json_str(&json).unwrap();
        assert_eq!(parsed.title.as_deref(), Some("axes"));
        assert_eq!(parsed.viewport.width, 320);
    }

    #[test]
    fn svg_renderer_projects_basic_primitives() {
        let scene = sample_scene().unwrap();
        let svg = render_svg(&scene).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<line"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("stroke=\"#dc2828\""));
    }

    #[test]
    fn validation_rejects_duplicate_ids() {
        let scene = SceneDocument::new(
            SceneViewport::new(320, 240).unwrap(),
            Camera::orthographic(Point3::new(3.0, 3.0, 3.0), Point3::new(0.0, 0.0, 0.0), 4.0)
                .unwrap(),
            Node::group([
                Node::point(Point3::new(0.0, 0.0, 0.0)).id("p"),
                Node::line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)).id("p"),
            ]),
        );
        assert!(scene.is_err());
    }

    fn sample_scene() -> Result<SceneDocument> {
        SceneDocument::new(
            SceneViewport::new(320, 240)?,
            Camera::orthographic(Point3::new(3.0, 3.0, 3.0), Point3::new(0.0, 0.0, 0.0), 4.0)?,
            Node::group([
                Node::line(Point3::new(-1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
                    .style(Style::stroke(ColorRgba::rgb(220, 40, 40), 2.0)),
                Node::point(Point3::new(0.0, 0.0, 0.0))
                    .style(Style::fill(ColorRgba::rgb(40, 120, 220))),
            ]),
        )
    }
}
