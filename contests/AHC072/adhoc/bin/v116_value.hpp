// Embedded by build_v116_guidance.py; the submission remains one C++ file.
namespace finite_value {
inline bool enabled=true;
inline int64_t calls=0;
/* V116_WEIGHTS */
inline float residual(const array<float,48>& x) {
    LOCAL_NOTE(++calls;)
    if(x[0]==0.0f)return -x[1]; // A zero lower bound here means the exact target state.
    float result=b2;
    for(int h=0;h<32;h++) {
        float z=b1[h];
        for(int j=0;j<48;j++)z+=w1[h][j]*x[j];
        result+=w2[h]*max(0.0f,z);
    }
    return result;
}
}
#ifdef V116_COLLECTOR
namespace finite_probe {
inline int width=12,node_limit=192;
inline function<bool(const FiniteState&,const FiniteState&,const vector<Move>&)> edge;
}
#endif
