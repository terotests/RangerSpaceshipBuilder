// SPDX-License-Identifier: MIT
// A small WebGL viewer for ship meshes: the 11-float corners ship_web gives
// (x y z, normal, linear colour, part, glow), lit the way RangerGames' engine
// lights a model (Lambert, a sky/ground ambient and one sun), an orbit camera,
// a grid, picking a part by its id, and several ships side by side (the
// scale view).

export class View3D {
  constructor(canvas) {
    this.canvas = canvas;
    const gl = canvas.getContext("webgl", { antialias: true, preserveDrawingBuffer: true });
    if (!gl) throw new Error("WebGL ei ole käytettävissä");
    this.gl = gl;
    this.items = [];          // { vbo, count, offset: [x, y, z], scale }
    this.selected = -1;
    this.yaw = 0.7; this.pitch = 0.35; this.dist = 9; this.target = [0, 0, 0];
    this.grid = true;
    this.onPick = null;
    this.prog = this.program(VS, FS);
    this.pickProg = this.program(VS, PICK_FS);
    this.lineProg = this.program(LINE_VS, LINE_FS);
    this.gridVbo = gl.createBuffer();
    this.gridCount = 0;
    this.makeGrid(1, 24);
    this.input();
    this.dirty = true;
    const loop = () => { if (this.dirty) { this.draw(); this.dirty = false; } requestAnimationFrame(loop); };
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

  /// One ship (the editor) or many (the scale view).
  setMeshes(list) {
    const gl = this.gl;
    for (const it of this.items) gl.deleteBuffer(it.vbo);
    this.items = list.map((m) => {
      const vbo = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
      gl.bufferData(gl.ARRAY_BUFFER, m.data, gl.STATIC_DRAW);
      return { vbo, count: m.data.length / 11, offset: m.offset || [0, 0, 0], scale: m.scale || 1, label: m.label };
    });
    this.dirty = true;
  }

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
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 44, off * 4);
    };
    attr("aPos", 3, 0); attr("aNrm", 3, 3); attr("aCol", 3, 6); attr("aPart", 1, 9); attr("aGlow", 1, 10);
  }

  draw() {
    const gl = this.gl;
    const { proj, view } = this.matrices();
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0.035, 0.05, 0.09, 1);
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
      for (let i = 0; i < 5; i++) gl.disableVertexAttribArray(i);
    }
    gl.useProgram(this.prog);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.prog, "uProj"), false, proj);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.prog, "uView"), false, view);
    gl.uniform1f(gl.getUniformLocation(this.prog, "uSel"), this.selected);
    for (const it of this.items) {
      this.bindMesh(this.prog, it);
      gl.uniform3fv(gl.getUniformLocation(this.prog, "uOff"), it.offset);
      gl.uniform1f(gl.getUniformLocation(this.prog, "uScale"), it.scale);
      gl.drawArrays(gl.TRIANGLES, 0, it.count);
    }
    for (let i = 0; i < 5; i++) gl.disableVertexAttribArray(i);
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
    for (let i = 0; i < 5; i++) gl.disableVertexAttribArray(i);
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

const VS = `
attribute vec3 aPos; attribute vec3 aNrm; attribute vec3 aCol; attribute float aPart; attribute float aGlow;
uniform mat4 uProj, uView; uniform vec3 uOff; uniform float uScale;
varying vec3 vNrm; varying vec3 vCol; varying float vPart; varying float vGlow; varying vec3 vPos;
void main() {
  vec3 p = aPos * uScale + uOff;
  vPos = p; vNrm = aNrm; vCol = aCol; vPart = aPart; vGlow = aGlow;
  gl_Position = uProj * uView * vec4(p, 1.0);
}`;

const FS = `
precision mediump float;
varying vec3 vNrm; varying vec3 vCol; varying float vPart; varying float vGlow; varying vec3 vPos;
uniform float uSel;
void main() {
  vec3 n = normalize(vNrm);
  vec3 sun = normalize(vec3(0.5, 0.8, 0.45));
  float hemi = 0.5 + 0.5 * n.y;
  vec3 amb = mix(vec3(0.10, 0.09, 0.08), vec3(0.32, 0.36, 0.44), hemi);
  vec3 lit = vCol * (amb + vec3(1.0, 0.97, 0.92) * max(dot(n, sun), 0.0) * 0.95);
  vec3 c = vGlow > 0.5 ? vCol : lit;
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
