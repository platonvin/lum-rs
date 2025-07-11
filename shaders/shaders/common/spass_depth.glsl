// subpass-loading related functions. Valid only in fragment (thus moved into separate file)

highp float load_depth_spass() {
    highp float depth_encoded = (subpassLoad(depthBuffer).x);
    return (depth_encoded) * 1000.0;
}