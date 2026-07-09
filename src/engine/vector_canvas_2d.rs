use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, LineCap, LineJoin, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct VectorPoint2D {
    pub x: f32,
    pub y: f32,
}

impl VectorPoint2D {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VectorPathCommand2D {
    MoveTo(VectorPoint2D),
    LineTo(VectorPoint2D),
    QuadraticTo {
        control: VectorPoint2D,
        to: VectorPoint2D,
    },
    CubicTo {
        control_a: VectorPoint2D,
        control_b: VectorPoint2D,
        to: VectorPoint2D,
    },
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorStyle2D {
    pub fill: Option<[u8; 4]>,
    pub stroke: Option<[u8; 4]>,
    pub stroke_width: f32,
    pub tolerance: f32,
    pub line_cap: VectorLineCap2D,
    pub line_join: VectorLineJoin2D,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VectorLineCap2D {
    #[default]
    Butt,
    Square,
    Round,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VectorLineJoin2D {
    Miter,
    #[default]
    Round,
    Bevel,
}

impl Default for VectorStyle2D {
    fn default() -> Self {
        Self {
            fill: Some([255, 255, 255, 255]),
            stroke: None,
            stroke_width: 1.0,
            tolerance: 0.1,
            line_cap: VectorLineCap2D::Round,
            line_join: VectorLineJoin2D::Round,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VectorPath2D {
    pub commands: Vec<VectorPathCommand2D>,
    pub style: VectorStyle2D,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VectorMesh2D {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VectorGeometry2D {
    pub fill: Option<VectorMesh2D>,
    pub stroke: Option<VectorMesh2D>,
}

impl VectorPath2D {
    pub fn new(style: VectorStyle2D) -> Self {
        Self {
            commands: Vec::new(),
            style,
        }
    }

    pub fn move_to(mut self, x: f32, y: f32) -> Self {
        self.commands
            .push(VectorPathCommand2D::MoveTo(VectorPoint2D::new(x, y)));
        self
    }

    pub fn line_to(mut self, x: f32, y: f32) -> Self {
        self.commands
            .push(VectorPathCommand2D::LineTo(VectorPoint2D::new(x, y)));
        self
    }

    pub fn quadratic_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.commands.push(VectorPathCommand2D::QuadraticTo {
            control: VectorPoint2D::new(cx, cy),
            to: VectorPoint2D::new(x, y),
        });
        self
    }

    pub fn cubic_to(mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) -> Self {
        self.commands.push(VectorPathCommand2D::CubicTo {
            control_a: VectorPoint2D::new(ax, ay),
            control_b: VectorPoint2D::new(bx, by),
            to: VectorPoint2D::new(x, y),
        });
        self
    }

    pub fn close(mut self) -> Self {
        self.commands.push(VectorPathCommand2D::Close);
        self
    }

    pub fn polygon(points: &[VectorPoint2D], style: VectorStyle2D) -> Self {
        let mut path = Self::new(style);
        if let Some(first) = points.first() {
            path.commands.push(VectorPathCommand2D::MoveTo(*first));
            path.commands.extend(
                points
                    .iter()
                    .skip(1)
                    .copied()
                    .map(VectorPathCommand2D::LineTo),
            );
            path.commands.push(VectorPathCommand2D::Close);
        }
        path
    }

    pub fn circle(center: VectorPoint2D, radius: f32, style: VectorStyle2D) -> Self {
        let r = radius.abs().max(0.001);
        let k = r * 0.552_284_8;
        Self::new(style)
            .move_to(center.x + r, center.y)
            .cubic_to(
                center.x + r,
                center.y + k,
                center.x + k,
                center.y + r,
                center.x,
                center.y + r,
            )
            .cubic_to(
                center.x - k,
                center.y + r,
                center.x - r,
                center.y + k,
                center.x - r,
                center.y,
            )
            .cubic_to(
                center.x - r,
                center.y - k,
                center.x - k,
                center.y - r,
                center.x,
                center.y - r,
            )
            .cubic_to(
                center.x + k,
                center.y - r,
                center.x + r,
                center.y - k,
                center.x + r,
                center.y,
            )
            .close()
    }

    pub fn rounded_rectangle(
        min: VectorPoint2D,
        max: VectorPoint2D,
        radius: f32,
        style: VectorStyle2D,
    ) -> Self {
        let width = (max.x - min.x).abs();
        let height = (max.y - min.y).abs();
        let r = radius.max(0.0).min(width * 0.5).min(height * 0.5);
        Self::new(style)
            .move_to(min.x + r, min.y)
            .line_to(max.x - r, min.y)
            .quadratic_to(max.x, min.y, max.x, min.y + r)
            .line_to(max.x, max.y - r)
            .quadratic_to(max.x, max.y, max.x - r, max.y)
            .line_to(min.x + r, max.y)
            .quadratic_to(min.x, max.y, min.x, max.y - r)
            .line_to(min.x, min.y + r)
            .quadratic_to(min.x, min.y, min.x + r, min.y)
            .close()
    }

    pub fn tessellate(&self) -> Result<VectorGeometry2D, String> {
        let lyon_path = self.build_lyon_path()?;
        let mut geometry = VectorGeometry2D::default();
        if let Some(color) = self.style.fill {
            let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
            FillTessellator::new()
                .tessellate_path(
                    &lyon_path,
                    &FillOptions::tolerance(self.style.tolerance.max(0.001)),
                    &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| {
                        vertex.position().to_array()
                    }),
                )
                .map_err(|error| format!("fill tessellation failed: {error:?}"))?;
            geometry.fill = Some(VectorMesh2D {
                vertices: buffers.vertices,
                indices: buffers.indices,
                color,
            });
        }
        if let Some(color) = self.style.stroke {
            let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
            let options = StrokeOptions::tolerance(self.style.tolerance.max(0.001))
                .with_line_width(self.style.stroke_width.max(0.1))
                .with_start_cap(line_cap(self.style.line_cap))
                .with_end_cap(line_cap(self.style.line_cap))
                .with_line_join(line_join(self.style.line_join));
            StrokeTessellator::new()
                .tessellate_path(
                    &lyon_path,
                    &options,
                    &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| {
                        vertex.position().to_array()
                    }),
                )
                .map_err(|error| format!("stroke tessellation failed: {error:?}"))?;
            geometry.stroke = Some(VectorMesh2D {
                vertices: buffers.vertices,
                indices: buffers.indices,
                color,
            });
        }
        Ok(geometry)
    }

    pub fn hit_test_fill(&self, point: VectorPoint2D) -> bool {
        self.tessellate()
            .ok()
            .and_then(|geometry| geometry.fill)
            .is_some_and(|mesh| mesh.hit_test(point))
    }

    fn build_lyon_path(&self) -> Result<Path, String> {
        let mut builder = Path::builder();
        let mut figure_open = false;
        for command in &self.commands {
            match command {
                VectorPathCommand2D::MoveTo(position) => {
                    if figure_open {
                        builder.end(false);
                    }
                    builder.begin(point(position.x, position.y));
                    figure_open = true;
                }
                VectorPathCommand2D::LineTo(position) if figure_open => {
                    builder.line_to(point(position.x, position.y));
                }
                VectorPathCommand2D::QuadraticTo { control, to } if figure_open => {
                    builder.quadratic_bezier_to(point(control.x, control.y), point(to.x, to.y));
                }
                VectorPathCommand2D::CubicTo {
                    control_a,
                    control_b,
                    to,
                } if figure_open => {
                    builder.cubic_bezier_to(
                        point(control_a.x, control_a.y),
                        point(control_b.x, control_b.y),
                        point(to.x, to.y),
                    );
                }
                VectorPathCommand2D::Close if figure_open => {
                    builder.end(true);
                    figure_open = false;
                }
                _ => return Err("path command requires an open figure".to_string()),
            }
        }
        if figure_open {
            builder.end(false);
        }
        Ok(builder.build())
    }
}

impl VectorMesh2D {
    pub fn hit_test(&self, point: VectorPoint2D) -> bool {
        self.indices.chunks_exact(3).any(|triangle| {
            point_in_triangle(
                [point.x, point.y],
                self.vertices[triangle[0] as usize],
                self.vertices[triangle[1] as usize],
                self.vertices[triangle[2] as usize],
            )
        })
    }
}

pub fn translation_gizmo(origin: VectorPoint2D, size: f32) -> Vec<VectorPath2D> {
    let size = size.max(12.0);
    let stroke = |color| VectorStyle2D {
        fill: None,
        stroke: Some(color),
        stroke_width: 3.0,
        ..VectorStyle2D::default()
    };
    vec![
        VectorPath2D::new(stroke([255, 92, 104, 255]))
            .move_to(origin.x, origin.y)
            .line_to(origin.x + size, origin.y),
        VectorPath2D::polygon(
            &[
                VectorPoint2D::new(origin.x + size + 8.0, origin.y),
                VectorPoint2D::new(origin.x + size - 1.0, origin.y - 6.0),
                VectorPoint2D::new(origin.x + size - 1.0, origin.y + 6.0),
            ],
            VectorStyle2D {
                fill: Some([255, 92, 104, 255]),
                stroke: None,
                ..VectorStyle2D::default()
            },
        ),
        VectorPath2D::new(stroke([94, 226, 145, 255]))
            .move_to(origin.x, origin.y)
            .line_to(origin.x, origin.y - size),
        VectorPath2D::polygon(
            &[
                VectorPoint2D::new(origin.x, origin.y - size - 8.0),
                VectorPoint2D::new(origin.x - 6.0, origin.y - size + 1.0),
                VectorPoint2D::new(origin.x + 6.0, origin.y - size + 1.0),
            ],
            VectorStyle2D {
                fill: Some([94, 226, 145, 255]),
                stroke: None,
                ..VectorStyle2D::default()
            },
        ),
    ]
}

fn line_cap(cap: VectorLineCap2D) -> LineCap {
    match cap {
        VectorLineCap2D::Butt => LineCap::Butt,
        VectorLineCap2D::Square => LineCap::Square,
        VectorLineCap2D::Round => LineCap::Round,
    }
}

fn line_join(join: VectorLineJoin2D) -> LineJoin {
    match join {
        VectorLineJoin2D::Miter => LineJoin::Miter,
        VectorLineJoin2D::Round => LineJoin::Round,
        VectorLineJoin2D::Bevel => LineJoin::Bevel,
    }
}

fn point_in_triangle(point: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let sign = |p: [f32; 2], first: [f32; 2], second: [f32; 2]| {
        (p[0] - second[0]) * (first[1] - second[1]) - (first[0] - second[0]) * (p[1] - second[1])
    };
    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);
    let has_negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_negative && has_positive)
}
