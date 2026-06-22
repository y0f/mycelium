use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ShaderEntry {
    pub name: String,
    pub source: String,
    pub description: String,
    /// path relative to working directory, for hot-reload
    pub path: String,
}

pub struct ShaderRegistry {
    shaders: HashMap<String, ShaderEntry>,
    order: Vec<String>,
}

impl ShaderRegistry {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn register(&mut self, name: &str, source: &str, description: &str, path: &str) {
        let entry = ShaderEntry {
            name: name.to_string(),
            source: source.to_string(),
            description: description.to_string(),
            path: path.to_string(),
        };
        if !self.shaders.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.shaders.insert(name.to_string(), entry);
    }

    pub fn load_builtins(&mut self) {
        self.register("fractal", include_str!("../../../assets/shaders/fractal.wgsl"),
            "Infinite space-folding fractal cosmos", "assets/shaders/fractal.wgsl");
        self.register("hypnotic", include_str!("../../../assets/shaders/warp_tunnel.wgsl"),
            "Concentric rings, spirals, Moire interference", "assets/shaders/warp_tunnel.wgsl");
        self.register("voronoi", include_str!("../../../assets/shaders/voronoi.wgsl"),
            "Organic animated Voronoi cellular patterns", "assets/shaders/voronoi.wgsl");
        self.register("kaleidoscope", include_str!("../../../assets/shaders/kaleidoscope.wgsl"),
            "Mirror folds over smooth layered noise", "assets/shaders/kaleidoscope.wgsl");
        self.register("neural", include_str!("../../../assets/shaders/reaction_diffusion.wgsl"),
            "Organic domain-warped flowing patterns", "assets/shaders/reaction_diffusion.wgsl");
        self.register("strobe", include_str!("../../../assets/shaders/strobe.wgsl"),
            "EPILEPTIC WARNING - aggressive beat-synced geometric flash", "assets/shaders/strobe.wgsl");
        self.register("geometry", include_str!("../../../assets/shaders/sdf_geometry.wgsl"),
            "Raymarched sacred geometry SDF with iridescent sheen", "assets/shaders/sdf_geometry.wgsl");
        self.register("nebula", include_str!("../../../assets/shaders/nebula.wgsl"),
            "Deep space gas clouds with volumetric layering", "assets/shaders/nebula.wgsl");
        self.register("electric", include_str!("../../../assets/shaders/electric.wgsl"),
            "Lightning bolts and plasma arcs", "assets/shaders/electric.wgsl");
        self.register("liquid", include_str!("../../../assets/shaders/liquid.wgsl"),
            "Flowing metallic fluid with domain warping", "assets/shaders/liquid.wgsl");
    }

    pub fn get(&self, name: &str) -> Option<&ShaderEntry> {
        self.shaders.get(name)
    }

    /// names in registration order
    pub fn names(&self) -> &[String] {
        &self.order
    }

    pub fn len(&self) -> usize {
        self.shaders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shaders.is_empty()
    }
}

impl Default for ShaderRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.load_builtins();
        reg
    }
}
