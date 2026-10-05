#!/usr/bin/env python3
"""行ごとに8bitへ丸めた重みを、単一C++へ埋め込む。バイアスはfloat32。"""
import argparse
import base64
import json
from pathlib import Path
import struct

import numpy as np

from v090_data import ROOT, RUN, MODEL_SPECS, save


def quantize(parameters):
    restored = {}; payload = bytearray(); layout = []
    for name, values in parameters.items():
        values = np.asarray(values, np.float32)
        shape = values.shape
        if name.endswith('.weight'):
            matrix = values.reshape(shape[0], -1)
            scale = np.max(np.abs(matrix), axis=1).astype(np.float32) / np.float32(127)
            scale[scale == 0] = 1.
            integers = np.clip(np.rint(matrix / scale[:, None]), -127, 127).astype(np.int8)
            restored[name] = (integers.astype(np.float32) * scale[:, None]).reshape(shape)
            for factor, row in zip(scale, integers):
                payload.extend(struct.pack('<f', factor)); payload.extend(row.tobytes())
            layout.append((name, matrix.shape[0], matrix.shape[1], True))
        else:
            restored[name] = values.copy(); payload.extend(values.astype('<f4').tobytes())
            layout.append((name, 1, values.size, False))
    return restored, bytes(payload), layout


def name_cpp(name):
    names = {'input.weight': 'nn_input_w', 'input.bias': 'nn_input_b',
             'actor.0.weight': 'nn_actor_w', 'actor.0.bias': 'nn_actor_b',
             'actor.2.weight': 'nn_actor_out_w', 'actor.2.bias': 'nn_actor_out_b',
             'critic.0.weight': 'nn_critic_w', 'critic.0.bias': 'nn_critic_b',
             'critic.2.weight': 'nn_critic_out_w', 'critic.2.bias': 'nn_critic_out_b'}
    if name in names: return names[name]
    _, block, part, kind = name.split('.')
    return f'nn_{part}_{"w" if kind == "weight" else "b"}[{block}]'


def build(model_path, destination, stochastic=False, diagnostic=False):
    model = json.loads(model_path.read_text()); p = model['parameters']; spec = model['spec']
    restored, payload, layout = quantize(p)
    C, H, D = spec['width'], spec['hidden'], spec['depth']
    parts = [(ROOT / 'adhoc/scripts/v089_core.cpp.txt').read_text(),
             f'// epoch={model["epoch"]}, dataset SHA256={model["dataset_sha256"]}\n',
             f'constexpr int NN_WIDTH={C}, NN_HIDDEN={H}, NN_DEPTH={D}, NN_ACTOR_INPUT={3*C+16};\n']
    if stochastic: parts.append('#define V090_STOCHASTIC\n')
    if diagnostic: parts.append('#define V090_DIAGNOSTIC\n')
    for name, values in p.items():
        if name.startswith('blocks.'):
            if not name.startswith('blocks.0.'): continue
            cpp = name_cpp(name).split('[')[0]; size = np.asarray(values).size
            parts.append(f'static array<array<float,{size}>,NN_DEPTH> {cpp};\n')
        else:
            parts.append(f'static array<float,{np.asarray(values).size}> {name_cpp(name)};\n')
    encoded = base64.b64encode(payload).decode('ascii')
    parts.append('static constexpr char nn_packed[] =\n')
    parts.extend('"' + encoded[i:i + 120] + '"\n' for i in range(0, len(encoded), 120))
    parts.append(';\n')
    parts.append('''
void load_nn() {
    array<int,256> lookup; lookup.fill(-1);
    const string alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for(int i=0;i<64;++i) lookup[uint8_t(alphabet[i])]=i;
    vector<uint8_t> bytes; bytes.reserve(sizeof(nn_packed)*3/4);
    unsigned buffer=0; int bits=0;
    for(char ch:nn_packed) {
        if(ch=='=' || ch==0) break;
        const int digit=lookup[uint8_t(ch)];
        if(digit<0) throw runtime_error("bad model encoding");
        buffer=(buffer<<6)|unsigned(digit); bits+=6;
        if(bits>=8) { bits-=8; bytes.push_back(uint8_t(buffer>>bits)); }
    }
    size_t pos=0;
    auto scalar=[&]() {
        if(pos+4>bytes.size()) throw runtime_error("short model scale");
        uint32_t value=0; for(int b=0;b<4;++b)value|=uint32_t(bytes[pos++])<<(8*b);
        return bit_cast<float>(value);
    };
    auto matrix=[&](float* out,int rows,int columns) {
        for(int row=0;row<rows;++row) {
            const float scale=scalar();
            for(int col=0;col<columns;++col) {
                if(pos==bytes.size())throw runtime_error("short model row");
                const int raw=bytes[pos++];
                *out++=float(raw<128?raw:raw-256)*scale;
            }
        }
    };
    auto bias=[&](float* out,int count){for(int i=0;i<count;++i)*out++=scalar();};
''')
    for name, rows, columns, quantized in layout:
        target = name_cpp(name)
        parts.append(f'    matrix({target}.data(),{rows},{columns});\n' if quantized
                     else f'    bias({target}.data(),{columns});\n')
    parts.append(f'    if(pos!=bytes.size() || pos!={len(payload)})throw runtime_error("trailing model data");\n}}\n')
    parts.append((ROOT / 'adhoc/scripts/v090_inference.cpp.txt').read_text())
    parts.append((ROOT / 'adhoc/scripts/v090_search.cpp.txt').read_text())
    text = ''.join(parts).splitlines(); text[0] = '// ' + destination.name
    destination.parent.mkdir(parents=True, exist_ok=True)
    source = '\n'.join(text) + '\n'
    if len(source.encode()) > 512000: raise RuntimeError(f'submission source too large: {len(source.encode())}')
    destination.write_text(source)
    return {'source_bytes': len(source.encode()), 'payload_bytes': len(payload),
            'parameters': sum(v.size for v in restored.values()), 'spec': spec,
            'quantization': 'int8 per output row, float32 bias and scales'}


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--model', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True); p.add_argument('--stochastic', action='store_true')
    p.add_argument('--diagnostic', action='store_true'); args = p.parse_args()
    print(json.dumps(build(args.model, args.output, args.stochastic, args.diagnostic)))
