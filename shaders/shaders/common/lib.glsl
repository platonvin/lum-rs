vec3 decode_color(vec3 encoded_color) {
    return encoded_color * COLOR_ENCODE_VALUE;
}
vec3 encode_color(vec3 color) {
    return color / COLOR_ENCODE_VALUE;
}

//not really nextafter but somewhat close
float next_after(float x, int s) {
    int ix = floatBitsToInt(x);
    float fxp1 = intBitsToFloat(ix + s);
    return fxp1;
}
float next_after(float x) {
    return next_after(x, 1);
}
float prev_befor(float x, int s) {
    int ix = floatBitsToInt(x);
    float fxp1 = intBitsToFloat(ix - s);
    return fxp1;
}
float prev_befor(float x) {
    return prev_befor(x, 1);
}

float square(float a) {
    return a * a;
}