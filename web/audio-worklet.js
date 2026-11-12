// The game's audio output (WEB-AUDIO): an AudioWorkletProcessor that plays the DirectSound mix sa-game's worker
// produces (interleaved stereo 16-bit at 44.1 kHz; the AudioContext runs at 44.1 kHz, the browser resamples to the
// device).
//
// Shared ring buffer (cross-origin isolated pages): header Int32Array [write, read, running, underruns, target, -, -,
// -] counted in frames (wrapping 32-bit), then the samples. The worker writes and tops it up to its target fill each main-loop
// iteration; this processor reads 128 frames per quantum and advances `read`, which is the play cursor the game's
// DirectSound buffers follow.
// Fallback (no SharedArrayBuffer): the page forwards the worker's chunks through the port into a local queue.
class SaGameAudio extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const sab = options.processorOptions?.sab;
    if (sab) {
      this.hdr = new Int32Array(sab, 0, 8);
      this.data = new Int16Array(sab, 32);
      this.cap = this.data.length / 2;
    } else {
      this.queue = [];
      this.queued = 0;
      this.port.onmessage = (e) => {
        const pcm = e.data;
        this.queue.push({ pcm, at: 0 });
        this.queued += pcm.length / 2;
        // more than half a second behind: drop the oldest (no back-pressure without shared memory)
        while (this.queued > 22050 && this.queue.length > 1) {
          const q = this.queue.shift();
          this.queued -= (q.pcm.length - q.at) / 2;
        }
      };
    }
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    const l = out[0];
    const r = out[1] ?? out[0];
    const n = l.length;
    let i = 0;
    if (this.hdr) {
      const w = Atomics.load(this.hdr, 0);
      let rd = Atomics.load(this.hdr, 1);
      const avail = (w - rd) | 0;
      const take = Math.max(0, Math.min(n, avail));
      for (; i < take; i++) {
        const p = ((rd + i) >>> 0) % this.cap;
        l[i] = this.data[p * 2] / 32768;
        r[i] = this.data[p * 2 + 1] / 32768;
      }
      rd = (rd + take) | 0;
      Atomics.store(this.hdr, 1, rd);
      if (take < n) Atomics.add(this.hdr, 3, 1);
    } else {
      while (i < n && this.queue.length) {
        const q = this.queue[0];
        while (i < n && q.at < q.pcm.length) {
          l[i] = q.pcm[q.at] / 32768;
          r[i] = q.pcm[q.at + 1] / 32768;
          q.at += 2;
          i++;
          this.queued--;
        }
        if (q.at >= q.pcm.length) this.queue.shift();
      }
    }
    for (; i < n; i++) {
      l[i] = 0;
      r[i] = 0;
    }
    return true;
  }
}

registerProcessor('sa-game-audio', SaGameAudio);
