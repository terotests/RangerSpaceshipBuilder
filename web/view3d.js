// SPDX-License-Identifier: MIT
// A small WebGL viewer for ship meshes: the 14-float corners ship_web gives
// (x y z, normal, linear colour, uv, part, glow, material), shaded with the
// procedural material maps (metal/roughness, normal map) under a choice of
// lights that can turn round the ship, an orbit camera,
// a grid, picking a part by its id, and several ships side by side (the
// scale view).

export class View3D {
  constructor(canvas) {
    this.canvas = canvas;
    const gl = canvas.getContext("webgl", { antialias: true, preserveDrawingBuffer: true });
    if (!gl) throw new Error("WebGL ei ole käytettävissä");
    this.gl = gl;
    this.derivs = !!gl.getExtension("OES_standard_derivatives");
    this.aniso = gl.getExtension("EXT_texture_filter_anisotropic");
    this.lights = LIGHTS.sun;
    this.lightSpin = false;
    this.lightAngle = 0;
    this.textures = true;
    this.texCache = new Map();
    this.white = this.makeTex(1, new Uint8Array([255, 255, 255]));
    this.flatOrm = this.makeTex(1, new Uint8Array([255, 255, 255]));
    this.flatNrm = this.makeTex(1, new Uint8Array([128, 128, 255]));
    this.items = [];          // { vbo, count, offset: [x, y, z], scale }
    this.selected = -1;
    this.yaw = 0.7; this.pitch = 0.35; this.dist = 9; this.target = [0, 0, 0];
    this.grid = true;
    this.onPick = null;
    this.prog = this.program(VS, (this.derivs ? "#extension GL_OES_standard_derivatives : enable\n#define DERIVS 1\n" : "") + FS);
    this.pickProg = this.program(VS, PICK_FS);
    this.lineProg = this.program(LINE_VS, LINE_FS);
    this.gridVbo = gl.createBuffer();
    this.gridCount = 0;
    this.makeGrid(1, 24);
    this.input();
    this.dirty = true;
    let last = performance.now();
    const loop = (now) => {
      const dt = Math.min(0.1, (now - last) / 1000); last = now;
      if (this.lightSpin) { this.lightAngle += dt * 0.6; this.dirty = true; }
      if (this.dirty) { this.draw(); this.dirty = false; }
      requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
    new ResizeObserver(() => { this.dirty = true; }).observe(canvas);
  }

  program(vs, fs) {
    const gl = this.gl;
    const sh = (type, src) => {
      const s = gl.createShader(type);
      gl.shaderSource(s, src); gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
      return s;
    };
    const p = gl.createProgram();
    gl.attachShader(p, sh(gl.VERTEX_SHADER, vs));
    gl.attachShader(p, sh(gl.FRAGMENT_SHADER, fs));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
    return p;
  }

  makeGrid(step, n) {
    const v = [];
    for (let i = -n; i <= n; i++) {
      v.push(i * step, 0, -n * step, i * step, 0, n * step);
      v.push(-n * step, 0, i * step, n * step, 0, i * step);
    }
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.gridVbo);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(v), gl.STATIC_DRAW);
    this.gridCount = v.length / 3;
    this.gridStep = step;
  }

  makeTex(size, rgb) {
    const gl = this.gl;
    const t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, size, size, 0, gl.RGB, gl.UNSIGNED_BYTE, rgb);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
    if (size > 1) {
      gl.generateMipmap(gl.TEXTURE_2D);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
      if (this.aniso) gl.texParameterf(gl.TEXTURE_2D, this.aniso.TEXTURE_MAX_ANISOTROPY_EXT, 8);
    } else {
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    }
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, size > 1 ? gl.LINEAR : gl.NEAREST);
    return t;
  }

  /// The GL textures of one material's maps, kept by `key` (the material's
  /// definition) so a rebuild with the same materials uploads nothing.
  materialTex(key, make) {
    let t = this.texCache.get(key);
    if (!t) {
      const m = make();
      if (!m) return null;
      const n = m.size * m.size * 3;
      t = { base: this.makeTex(m.size, m.data.subarray(0, n)), orm: this.makeTex(m.size, m.data.subarray(n, 2 * n)), normal: this.makeTex(m.size, m.data.subarray(2 * n, 3 * n)) };
      this.texCache.set(key, t);
    }
    return t;
  }

  /// One ship (the editor) or many (the scale view): { data, ranges, mats, offset, scale }.
  setMeshes(list) {
    const gl = this.gl;
    for (const it of this.items) gl.deleteBuffer(it.vbo);
    this.items = list.map((m) => {
      const vbo = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
      gl.bufferData(gl.ARRAY_BUFFER, m.data, gl.STATIC_DRAW);
      const count = m.data.length / STRIDE;
      return { vbo, count, ranges: m.ranges || [[0, count]], mats: m.mats || [], offset: m.offset || [0, 0, 0], scale: m.scale || 1, label: m.label };
    });
    this.dirty = true;
  }

  setLights(name) { this.lights = LIGHTS[name] || LIGHTS.sun; this.dirty = true; }

  /// Frames the camera on a box [min, max].
  frame(min, max, keepAngle) {
    const c = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2];
    const r = Math.max(0.5, Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) / 2);
    this.target = c;
    this.dist = r * 2.6;
    if (!keepAngle) { this.yaw = 0.75; this.pitch = 0.38; }
    // a grid step that suits the size: 1, 2, 5, 10, …
    const want = r / 6;
    const p = Math.pow(10, Math.floor(Math.log10(want)));
    const step = want / p < 2 ? p : want / p < 5 ? 2 * p : 5 * p;
    if (step !== this.gridStep) this.makeGrid(step, 24);
    this.gridY = min[1] - r * 0.15;
    this.dirty = true;
  }

  matrices() {
    const w = this.canvas.clientWidth, h = this.canvas.clientHeight;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    if (this.canvas.width !== Math.round(w * dpr) || this.canvas.height !== Math.round(h * dpr)) {
      this.canvas.width = Math.round(w * dpr); this.canvas.height = Math.round(h * dpr);
    }
    const cp = Math.cos(this.pitch), sp = Math.sin(this.pitch);
    const eye = [
      this.target[0] + this.dist * cp * Math.sin(this.yaw),
      this.target[1] + this.dist * sp,
      this.target[2] + this.dist * cp * Math.cos(this.yaw),
    ];
    const near = Math.max(0.01, this.dist * 0.01), far = this.dist * 50;
    const proj = perspective(45 * Math.PI / 180, w / Math.max(1, h), near, far);
    const view = lookAt(eye, this.target, [0, 1, 0]);
    return { proj, view, eye };
  }

  bindMesh(prog, it) {
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, it.vbo);
    const attr = (name, size, off) => {
      const loc = gl.getAttribLocation(prog, name);
      if (loc < 0) return;
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, STRIDE * 4, off * 4);
    };
    attr("aPos", 3, 0); attr("aNrm", 3, 3); attr("aCol", 3, 6); attr("aUv", 2, 9); attr("aPart", 1, 11); attr("aGlow", 1, 12);
  }

  draw() {
    const gl = this.gl;
    const { proj, view, eye } = this.matrices();
    const L = this.lights;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(L.bg[0], L.bg[1], L.bg[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    gl.enable(gl.CULL_FACE);
    if (this.grid) {
      gl.useProgram(this.lineProg);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.gridVbo);
      const loc = gl.getAttribLocation(this.lineProg, "aPos");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 3, gl.FLOAT, false, 12, 0);
      gl.uniformMatrix4fv(gl.getUniformLocation(this.lineProg, "uProj"), false, proj);
      gl.uniformMatrix4fv(gl.getUniformLocation(this.lineProg, "uView"), false, view);
      gl.uniform1f(gl.getUniformLocation(this.lineProg, "uR"), this.gridStep * 24);
      gl.uniform3f(gl.getUniformLocation(this.lineProg, "uOff"), this.target[0], this.gridY ?? -1, this.target[2]);
      gl.drawArrays(gl.LINES, 0, this.gridCount);
      for (let i = 0; i < 6; i++) gl.disableVertexAttribArray(i);
    }
    const P = this.prog;
    const u = (n) => gl.getUniformLocation(P, n);
    gl.useProgram(P);
    gl.uniformMatrix4fv(u("uProj"), false, proj);
    gl.uniformMatrix4fv(u("uView"), false, view);
    gl.uniform1f(u("uSel"), this.selected);
    gl.uniform3fv(u("uEye"), eye);
    // the key light can turn round the ship (the light demo)
    const a = this.lightAngle;
    const rot = (d) => [d[0] * Math.cos(a) + d[2] * Math.sin(a), d[1], -d[0] * Math.sin(a) + d[2] * Math.cos(a)];
    gl.uniform3fv(u("uKeyDir"), norm(rot(L.key.dir)));
    gl.uniform3fv(u("uKeyCol"), L.key.col.map((c) => c * L.key.int));
    gl.uniform3fv(u("uFillDir"), norm(rot(L.fill.dir)));
    gl.uniform3fv(u("uFillCol"), L.fill.col.map((c) => c * L.fill.int));
    gl.uniform3fv(u("uSky"), L.sky);
    gl.uniform3fv(u("uGround"), L.ground);
    gl.uniform1f(u("uExposure"), L.exposure || 1);
    gl.uniform1f(u("uTextured"), this.textures ? 1 : 0);
    gl.uniform1i(u("uBase"), 0); gl.uniform1i(u("uOrm"), 1); gl.uniform1i(u("uNrm"), 2);
    for (const it of this.items) {
      this.bindMesh(P, it);
      gl.uniform3fv(u("uOff"), it.offset);
      gl.uniform1f(u("uScale"), it.scale);
      it.ranges.forEach(([first, count], mi) => {
        if (!count) return;
        const m = it.mats[mi] || {};
        const t = this.textures ? m.tex : null;
        gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, t ? t.base : this.white);
        gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, t ? t.orm : this.flatOrm);
        gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, t ? t.normal : this.flatNrm);
        gl.uniform1f(u("uMetal"), t ? 1 : (m.metal ?? 0.3));
        gl.uniform1f(u("uRough"), t ? 1 : (m.rough ?? 0.5));
        gl.drawArrays(gl.TRIANGLES, first, count);
      });
    }
    gl.activeTexture(gl.TEXTURE0);
    for (let i = 0; i < 6; i++) gl.disableVertexAttribArray(i);
  }

  /// The part under (x, y) (CSS pixels in the canvas), -1 for none.
  pick(x, y) {
    if (this.items.length !== 1) return -1;
    const gl = this.gl;
    const { proj, view } = this.matrices();
    const w = this.canvas.width, h = this.canvas.height;
    if (!this.fb || this.fbW !== w || this.fbH !== h) {
      this.fb = gl.createFramebuffer();
      const tex = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      const rb = gl.createRenderbuffer();
      gl.bindRenderbuffer(gl.RENDERBUFFER, rb);
      gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT16, w, h);
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER, rb);
      this.fbW = w; this.fbH = h;
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
    gl.viewport(0, 0, w, h);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.useProgram(this.pickProg);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.pickProg, "uProj"), false, proj);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.pickProg, "uView"), false, view);
    const it = this.items[0];
    this.bindMesh(this.pickProg, it);
    gl.uniform3fv(gl.getUniformLocation(this.pickProg, "uOff"), it.offset);
    gl.uniform1f(gl.getUniformLocation(this.pickProg, "uScale"), it.scale);
    gl.drawArrays(gl.TRIANGLES, 0, it.count);
    for (let i = 0; i < 6; i++) gl.disableVertexAttribArray(i);
    const dpr = w / this.canvas.clientWidth;
    const px = new Uint8Array(4);
    gl.readPixels(Math.round(x * dpr), Math.round(h - y * dpr), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    this.dirty = true;
    if (px[3] === 0) return -1;
    return px[0] + px[1] * 256 - 1;
  }

  input() {
    const c = this.canvas;
    const pointers = new Map();
    let moved = 0, pinch = 0;
    c.addEventListener("contextmenu", (e) => e.preventDefault());
    c.addEventListener("pointerdown", (e) => {
      c.setPointerCapture(e.pointerId);
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY, button: e.button, shift: e.shiftKey });
      moved = 0;
      if (pointers.size === 2) { const [a, b] = [...pointers.values()]; pinch = Math.hypot(a.x - b.x, a.y - b.y); }
    });
    c.addEventListener("pointermove", (e) => {
      const p = pointers.get(e.pointerId);
      if (!p) return;
      const dx = e.clientX - p.x, dy = e.clientY - p.y;
      moved += Math.abs(dx) + Math.abs(dy);
      if (pointers.size === 2) {
        p.x = e.clientX; p.y = e.clientY;
        const [a, b] = [...pointers.values()];
        const d = Math.hypot(a.x - b.x, a.y - b.y);
        if (pinch > 0) this.dist = Math.max(0.2, this.dist * pinch / d);
        pinch = d;
      } else if (p.button === 2 || p.button === 1 || p.shift) {
        this.pan(dx, dy);
        p.x = e.clientX; p.y = e.clientY;
      } else {
        this.yaw -= dx * 0.008;
        this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + dy * 0.008));
        p.x = e.clientX; p.y = e.clientY;
      }
      this.dirty = true;
    });
    const up = (e) => {
      const p = pointers.get(e.pointerId);
      pointers.delete(e.pointerId);
      if (p && moved < 4 && pointers.size === 0 && this.onPick) {
        const r = c.getBoundingClientRect();
        this.onPick(this.pick(e.clientX - r.left, e.clientY - r.top));
      }
    };
    c.addEventListener("pointerup", up);
    c.addEventListener("pointercancel", (e) => pointers.delete(e.pointerId));
    c.addEventListener("wheel", (e) => {
      e.preventDefault();
      this.dist = Math.max(0.2, this.dist * Math.exp(e.deltaY * 0.0012));
      this.dirty = true;
    }, { passive: false });
  }

  pan(dx, dy) {
    const k = this.dist * 0.0015;
    const right = [Math.cos(this.yaw), 0, -Math.sin(this.yaw)];
    const up = [-Math.sin(this.pitch) * Math.sin(this.yaw), Math.cos(this.pitch), -Math.sin(this.pitch) * Math.cos(this.yaw)];
    for (let i = 0; i < 3; i++) this.target[i] += (-right[i] * dx + up[i] * dy) * k;
  }

  /// A PNG of the view (data URL), for thumbnails.
  snapshot() { this.draw(); return this.canvas.toDataURL("image/png"); }
}

const STRIDE = 14;

/// Light setups: a key light, a fill light, the sky and ground of the
/// ambient light (also what shiny surfaces reflect), and the background.
export const LIGHTS = {
  sun: { label: "Aurinko", key: { dir: [0.5, 0.75, 0.45], col: [1, 0.96, 0.9], int: 3.0 }, fill: { dir: [-0.6, -0.2, -0.5], col: [0.45, 0.6, 1], int: 0.35 },
    sky: [0.2, 0.24, 0.32], ground: [0.05, 0.045, 0.04], bg: [0.035, 0.05, 0.09], exposure: 1 },
  red: { label: "Punainen jättiläinen", key: { dir: [0.9, 0.15, 0.3], col: [1, 0.42, 0.18], int: 3.4 }, fill: { dir: [-0.7, 0.3, -0.6], col: [0.2, 0.75, 1], int: 1.4 },
    sky: [0.12, 0.05, 0.05], ground: [0.02, 0.03, 0.06], bg: [0.07, 0.025, 0.03], exposure: 1.05 },
  nebula: { label: "Sumu", key: { dir: [-0.4, 0.6, 0.6], col: [0.75, 0.45, 1], int: 2.2 }, fill: { dir: [0.7, -0.3, -0.4], col: [0.2, 1, 0.8], int: 1.2 },
    sky: [0.22, 0.12, 0.3], ground: [0.03, 0.12, 0.12], bg: [0.06, 0.03, 0.09], exposure: 1.1 },
  dock: { label: "Telakka", key: { dir: [0.1, 1, 0.2], col: [1, 1, 1], int: 2.6 }, fill: { dir: [0.3, 0.2, 1], col: [0.9, 0.95, 1], int: 0.9 },
    sky: [0.38, 0.4, 0.42], ground: [0.12, 0.12, 0.12], bg: [0.13, 0.14, 0.16], exposure: 1 },
  rim: { label: "Varjopuoli", key: { dir: [-0.3, 0.35, -1], col: [0.85, 0.9, 1], int: 4.5 }, fill: { dir: [0.5, -1, 0.3], col: [0.25, 0.15, 0.1], int: 0.4 },
    sky: [0.03, 0.035, 0.05], ground: [0.01, 0.01, 0.012], bg: [0.01, 0.012, 0.02], exposure: 1.2 },
};

const VS = `
attribute vec3 aPos; attribute vec3 aNrm; attribute vec3 aCol; attribute vec2 aUv; attribute float aPart; attribute float aGlow;
uniform mat4 uProj, uView; uniform vec3 uOff; uniform float uScale;
varying vec3 vNrm; varying vec3 vCol; varying vec2 vUv; varying float vPart; varying float vGlow; varying vec3 vPos;
void main() {
  vec3 p = aPos * uScale + uOff;
  vPos = p; vNrm = aNrm; vCol = aCol; vUv = aUv; vPart = aPart; vGlow = aGlow;
  gl_Position = uProj * uView * vec4(p, 1.0);
}`;

// Metal/roughness shading (GGX, Schlick) with two lights and a sky/ground
// ambient that shiny surfaces reflect; the normal map is applied in a
// tangent frame made from screen-space derivatives (no tangents needed).
const FS = `
precision highp float;
varying vec3 vNrm; varying vec3 vCol; varying vec2 vUv; varying float vPart; varying float vGlow; varying vec3 vPos;
uniform float uSel, uMetal, uRough, uExposure, uTextured;
uniform vec3 uEye, uKeyDir, uKeyCol, uFillDir, uFillCol, uSky, uGround;
uniform sampler2D uBase, uOrm, uNrm;
vec3 srgb2lin(vec3 c) { return pow(c, vec3(2.2)); }
vec3 light(vec3 n, vec3 v, vec3 l, vec3 col, vec3 albedo, float metal, float rough) {
  vec3 h = normalize(l + v);
  float nl = max(dot(n, l), 0.0), nv = max(dot(n, v), 0.001), nh = max(dot(n, h), 0.0), vh = max(dot(v, h), 0.0);
  float a = rough * rough, a2 = a * a;
  float d = a2 / (3.14159 * pow(nh * nh * (a2 - 1.0) + 1.0, 2.0));
  float k = (rough + 1.0) * (rough + 1.0) / 8.0;
  float g = nl / (nl * (1.0 - k) + k) * nv / (nv * (1.0 - k) + k);
  vec3 f0 = mix(vec3(0.04), albedo, metal);
  vec3 f = f0 + (1.0 - f0) * pow(1.0 - vh, 5.0);
  vec3 spec = d * g * f / max(4.0 * nl * nv, 0.001);
  vec3 diff = (1.0 - f) * (1.0 - metal) * albedo / 3.14159;
  return (diff + spec) * col * nl;
}
void main() {
  vec3 n = normalize(vNrm);
  vec3 v = normalize(uEye - vPos);
  vec3 albedo = vCol * srgb2lin(texture2D(uBase, vUv).rgb);
  vec3 orm = texture2D(uOrm, vUv).rgb;
  float rough = clamp(orm.g * uRough, 0.04, 1.0);
  float metal = clamp(orm.b * uMetal, 0.0, 1.0);
#ifdef DERIVS
  if (uTextured > 0.5) {
    vec3 dp1 = dFdx(vPos), dp2 = dFdy(vPos);
    vec2 du1 = dFdx(vUv), du2 = dFdy(vUv);
    vec3 dp2perp = cross(dp2, n), dp1perp = cross(n, dp1);
    vec3 t = dp2perp * du1.x + dp1perp * du2.x;
    vec3 b = dp2perp * du1.y + dp1perp * du2.y;
    float inv = inversesqrt(max(dot(t, t), dot(b, b)) + 1e-12);
    vec3 m = texture2D(uNrm, vUv).xyz * 2.0 - 1.0;
    n = normalize(mat3(t * inv, -b * inv, n) * m);
  }
#endif
  vec3 c = light(n, v, normalize(uKeyDir), uKeyCol, albedo, metal, rough)
         + light(n, v, normalize(uFillDir), uFillCol, albedo, metal, rough);
  // ambient: the sky above, the ground below; metals reflect it
  vec3 r = reflect(-v, n);
  vec3 envDiff = mix(uGround, uSky, 0.5 + 0.5 * n.y);
  vec3 envSpec = mix(uGround, uSky * 1.6, smoothstep(-0.3, 0.6, r.y));
  vec3 f0 = mix(vec3(0.04), albedo, metal);
  float nv = max(dot(n, v), 0.0);
  vec3 fr = f0 + (max(vec3(1.0 - rough), f0) - f0) * pow(1.0 - nv, 5.0);
  c += envDiff * albedo * (1.0 - metal) + envSpec * fr * (1.0 - rough * 0.7);
  if (vGlow > 0.5) c = vCol * 1.4;
  c *= uExposure * 1.5;
  c = c / (1.0 + c * 0.25);                       // soft highlights
  if (uSel >= 0.0 && abs(vPart - uSel) < 0.5) c = mix(c, vec3(1.0, 0.75, 0.2), 0.35);
  gl_FragColor = vec4(pow(c, vec3(1.0 / 2.2)), 1.0);
}`;

const PICK_FS = `
precision mediump float;
varying float vPart;
void main() {
  float id = vPart + 1.0;
  gl_FragColor = vec4(mod(id, 256.0) / 255.0, floor(id / 256.0) / 255.0, 0.0, 1.0);
}`;

const LINE_VS = `
attribute vec3 aPos; uniform mat4 uProj, uView; uniform vec3 uOff; uniform float uR; varying float vFade;
void main() { vec3 p = aPos + uOff; vFade = 1.0 - clamp(length(aPos.xz) / uR, 0.0, 1.0); gl_Position = uProj * uView * vec4(p, 1.0); }`;

const LINE_FS = `
precision mediump float; varying float vFade;
void main() { gl_FragColor = vec4(vec3(0.25, 0.42, 0.6) * 0.6 * vFade, 1.0); }`;

function perspective(fov, aspect, near, far) {
  const f = 1 / Math.tan(fov / 2), nf = 1 / (near - far);
  return new Float32Array([f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) * nf, -1, 0, 0, 2 * far * near * nf, 0]);
}

function lookAt(eye, at, up) {
  const z = norm([eye[0] - at[0], eye[1] - at[1], eye[2] - at[2]]);
  const x = norm(cross(up, z));
  const y = cross(z, x);
  return new Float32Array([
    x[0], y[0], z[0], 0, x[1], y[1], z[1], 0, x[2], y[2], z[2], 0,
    -dot(x, eye), -dot(y, eye), -dot(z, eye), 1,
  ]);
}
function cross(a, b) { return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; }
function dot(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
function norm(a) { const l = Math.hypot(a[0], a[1], a[2]) || 1; return [a[0] / l, a[1] / l, a[2] / l]; }
