#!/usr/bin/env python3
"""Cutie mask propagation (low-VRAM).

Takes a video, a starting mask PNG (binary, 0/255), and an output directory.
Propagates the mask forward from the seed frame and writes one binary
mask PNG per frame plus a 'done' sentinel when finished.

Runs under torch.inference_mode() + autocast so an 8GB card can survive.
"""

import argparse
import sys
from pathlib import Path

import numpy as np
import torch
import imageio
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent / "Cutie"))
from cutie.inference.inference_core import InferenceCore
from cutie.utils.get_default_model import get_default_model


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--video", required=True)
    ap.add_argument("--mask", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed-frame", type=int, default=0)
    ap.add_argument("--device", default="cuda")
    ap.add_argument("--max-height", type=int, default=480)
    args = ap.parse_args()

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    use_cuda = args.device != "cpu" and torch.cuda.is_available()
    device = torch.device("cuda" if use_cuda else "cpu")
    print(f"[track] device={device}", flush=True)

    cutie = get_default_model()
    processor = InferenceCore(cutie, cfg=cutie.cfg)

    # Keep Cutie's internal working resolution bounded even if the caller
    # passes a larger frame.
    if hasattr(processor, "max_internal_size"):
        processor.max_internal_size = args.max_height

    # Cap the memory bank.  Long-term memory is enabled by default, and it
    # has its own frame budget.
    try:
        processor.memory_manager.max_mem_frames = 5
        if hasattr(processor.memory_manager, "long_term"):
            processor.memory_manager.long_term.max_mem_frames = 5
    except Exception:
        pass

    start = np.array(Image.open(args.mask).convert("L"))
    start = (start > 127).astype(np.int64)
    if start.sum() == 0:
        print("[track] starting mask is empty", file=sys.stderr)
        return 2

    reader = imageio.get_reader(args.video)
    meta = reader.get_meta_data()
    in_w, in_h = meta["size"]
    scale = min(1.0, args.max_height / in_h)
    out_w, out_h = int(in_w * scale), int(in_h * scale)

    if start.shape[0] != out_h or start.shape[1] != out_w:
        start_img = Image.fromarray((start * 255).astype(np.uint8))
        start_img = start_img.resize((out_w, out_h), Image.NEAREST)
        start = (np.array(start_img) > 127).astype(np.int64)

    print(f"[track] resized to {out_w}x{out_h}", flush=True)

    written = 0
    with torch.inference_mode():
        for i, frame in enumerate(reader):
            if i < args.seed_frame:
                continue

            pil = Image.fromarray(frame).resize((out_w, out_h), Image.BILINEAR)
            arr = np.array(pil)

            image = torch.from_numpy(arr).permute(2, 0, 1).float() / 255.0
            image = image.to(device)

            with torch.autocast(device_type="cuda", dtype=torch.float16, enabled=use_cuda):
                if i == args.seed_frame:
                    mask_t = torch.from_numpy(start).to(device).long()
                    prob = processor.step(image, mask_t, [1])
                else:
                    prob = processor.step(image)

            out = processor.output_prob_to_mask(prob)
            out = out.cpu().numpy()
            if out.ndim > 2:
                out = out.squeeze()
            bin_mask = (out > 0).astype(np.uint8) * 255
            Image.fromarray(bin_mask).save(out_dir / f"{i:05d}.png")
            written += 1

            if i % 50 == 0:
                torch.cuda.empty_cache()

            if i % 30 == 0:
                print(f"[track] frame {i}", flush=True)

    (out_dir / "done").write_text(f"{written}\n")
    print(f"[track] wrote {written} masks to {out_dir}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())