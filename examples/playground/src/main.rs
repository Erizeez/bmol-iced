use liquid_glass::{
    GlassContainer, GlassId, GlassMaterial, GlassScene, GlassShape, LiquidRenderer, Rect,
};

fn main() {
    let panel = GlassContainer::new(GlassId(1), Rect::new(80.0, 64.0, 480.0, 320.0))
        .shape(GlassShape::Superellipse { exponent: 4.5 })
        .material(GlassMaterial::regular())
        .padding(16.0);

    let mut scene = GlassScene::default();
    scene.push(panel.node().clone());

    let renderer = LiquidRenderer::new();
    println!("Liquid Glass foundation playground");
    println!("nodes: {}", scene.nodes().len());
    println!("capture region: {:?}", renderer.capture_region(&scene));
    println!("render graph: {:?}", renderer.graph().passes());
}
