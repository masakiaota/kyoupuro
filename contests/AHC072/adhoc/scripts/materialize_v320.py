#!/usr/bin/env python3
"""Reconstruct frozen v320. No training, solver execution or eval registration.
Read exact parents; refuse drift. All runtime data are inside the resulting CPP.
"""
from pathlib import Path
import re,difflib,hashlib,json
ROOT=Path(__file__).resolve().parents[2]
def sha(data):return hashlib.sha256(data).hexdigest()
EXPECTED='9230c3d0ee6ec5ad955f862b81eec70ce682b3f4087983631f67948ab60d23f8'
PARENTS={'v113': {'path': 'src/bin/v113_integrated_nn_lns.cpp', 'sha256': '037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'}, 'v315': {'path': 'src/bin/v315_reactive_racing.cpp', 'sha256': '364cab41a22ccb8c5d84bb0b74086dbb4d1ffd635ad735fcdf8231efa21e3538'}}
SOURCES={v:ROOT/info["path"] for v,info in PARENTS.items()}
MODEL={'versions': ['v111', 'v113', 'v315', 'v210'], 'tree': {'choice': 1, 'n': 320, 'loss': 343.18519472627236, 'feature': 15, 'threshold': 0.10574372577025097, 'left': {'choice': 2, 'n': 80, 'loss': 66.97515009280848, 'feature': -1}, 'right': {'choice': 1, 'n': 240, 'loss': 276.2100446334639, 'feature': -1}}}
FEATURE_CODE="// Version routing uses only the initial board, never its seed or saved answers.\nstatic std::array<double,17> v320_features(int N,int K,const std::vector<std::string>& C) {\n    int id[20][20];\n    for(auto& row:id)std::fill(std::begin(row),std::end(row),-1);\n    std::vector<std::pair<int,int>> cells;\n    for(int i=0;i<N;++i)for(int j=0;j<N;++j)if(C[i][j]!='#') {\n        id[i][j]=int(cells.size());cells.emplace_back(i,j);\n    }\n    const int F=int(cells.size());\n    std::array<int,12> nests{},counts{};\n    struct Token{int p,c,i,j;};std::vector<Token> tokens;\n    std::vector<std::vector<int>> adj(F);\n    const int di[4]={-1,1,0,0},dj[4]={0,0,-1,1};int rays=0;\n    for(int p=0;p<F;++p) {\n        const auto [i,j]=cells[p];const char c=C[i][j];\n        if(c>='A'&&c<='L')nests[c-'A']=p;\n        if(c>='a'&&c<='l'){tokens.push_back({p,c-'a',i,j});++counts[c-'a'];}\n        for(int d=0;d<4;++d)for(int l=1;l<=8;++l) {\n            const int x=i+di[d]*l,y=j+dj[d]*l;\n            if(x<0||x>=N||y<0||y>=N||id[x][y]<0)break;\n            if(l==1)adj[p].push_back(id[x][y]);\n            ++rays;\n        }\n    }\n    std::vector<std::vector<int>> distances(K,std::vector<int>(F,-1));\n    for(int c=0;c<K;++c) {\n        auto& ds=distances[c];std::vector<int> q;q.reserve(F);q.push_back(nests[c]);ds[nests[c]]=0;\n        for(size_t t=0;t<q.size();++t)for(int y:adj[q[t]])if(ds[y]<0){ds[y]=ds[q[t]]+1;q.push_back(y);}\n    }\n    const int M=int(tokens.size());int home=0,max_home=0,nest_sum=0,same=0,near_sum=0,near_count=0;\n    for(const auto& a:tokens) {\n        const int h=distances[a.c][a.p];home+=h;max_home=std::max(max_home,h);\n        bool has_same=false;\n        for(int q:adj[a.p])has_same|=C[cells[q].first][cells[q].second]==char('a'+a.c);\n        same+=has_same;int best=1000000;\n        for(const auto& b:tokens)if(a.p!=b.p&&a.c==b.c)best=std::min(best,std::abs(a.i-b.i)+std::abs(a.j-b.j));\n        if(best<1000000){near_sum+=best;++near_count;}\n    }\n    for(int a=0;a<K;++a)for(int b=a+1;b<K;++b)nest_sum+=distances[a][nests[b]];\n    int dead=0,degree=0,maximum=0,squares=0;\n    for(const auto& a:adj){dead+=a.size()==1;degree+=int(a.size());}\n    for(int c=0;c<K;++c){maximum=std::max(maximum,counts[c]);squares+=counts[c]*counts[c];}\n    return {double(N),double(K),double(M),double(F),double(M)/(F-K),1.0-double(F)/(N*N),\n        double(maximum)/M,double(squares)/(M*M),double(M)/K,double(home)/M,double(max_home),\n        double(nest_sum)/(K*(K-1)/2),double(dead)/F,double(degree)/F,double(rays)/(4*F),\n        double(same)/M,double(near_sum)/std::max(1,near_count)};\n}\n"

def strip_comments(s):
    # The four pinned parents have no raw strings or digit separators. Refuse
    # those syntax forms rather than risk changing a string or numeric token.
    if re.search(r'\bR"[^ ()\\\t\r\n]{0,16}\(',s) or re.search(r"\d'\d",s):
        raise ValueError('Unsupported lexical form in new parent')
    pattern=re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|//[^\n]*|/\*[\s\S]*?\*/')
    return pattern.sub(lambda m:' ' if m[0].startswith(('//','/*')) else m[0],s)

def balanced_fragments(lines):
    # Argument lists are collected before macro expansion. Never replace a
    # span containing only half of a LOCAL_* invocation or a delimiter pair.
    literal=re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'')
    result=[];run=[];stack=[];last=0
    pairs={')':'(',']':'[','}':'{'}
    for line in lines:
        plain=literal.sub('literal',line);invalid=False
        trial=stack.copy()
        for c in plain:
            if c in '([{':trial.append(c)
            elif c in ')]}':
                if not trial or trial[-1]!=pairs[c]:invalid=True;break
                trial.pop()
            elif c==',' and not trial:invalid=True;break
        if invalid:
            value=''.join(run[:last])
            if len(value)>=600:result.append(value)
            run=[];stack=[];last=0
            continue
        run.append(line);stack=trial
        if not stack:last=len(run)
        if not stack and sum(map(len,run))>=24000:
            value=''.join(run)
            if len(value)>=600:result.append(value)
            run=[];last=0
    value=''.join(run[:last])
    if len(value)>=600:result.append(value)
    return result

def factor(text):
    # Include directives remain outside namespaces. Each parent block retains
    # its own #if/#define/#pragma sequence. Only directive-free spans are shared.
    prefix,rest=text.split('// ===== frozen ',1)
    blocks=['// ===== frozen '+p for p in rest.split('// ===== frozen ')]
    clean=[]
    for b in blocks:
        x=strip_comments(b)
        x='\n'.join(l.strip() for l in x.splitlines() if l.strip())+'\n'
        clean.append(x)
    lines=[s.splitlines(True) for s in clean];fragments=set()
    for i in range(len(lines)):
        for j in range(i):
            for m in difflib.SequenceMatcher(None,lines[i],lines[j],autojunk=True).get_matching_blocks():
                run=[];size=0
                def flush():
                    fragments.update(balanced_fragments(run))
                for line in lines[i][m.a:m.a+m.size]:
                    if line.lstrip().startswith('#') or line.rstrip().endswith('\\'):
                        flush();run=[];size=0
                    else:
                        if size+len(line)>32000:flush();run=[];size=0
                        run.append(line);size+=len(line)
                flush()
    definitions=[];stats=[]
    for frag in sorted(fragments,key=lambda s:(-len(s),s)):
        count=sum(s.count(frag) for s in clean)
        if count<2:continue
        name=f'V320_SHARED_{len(definitions)}'
        value=' '.join(l.strip() for l in frag.splitlines())
        definitions.append(f'#define {name} {value}\n')
        clean=[s.replace(frag,name+'\n') for s in clean]
        stats.append({'macro':name,'copies':count,'original_bytes':len(frag)})
    commentary='// Repeated parent code is factored into macros to keep one small CPP.\n// The unfactored form and token-equivalence check are in the experiment bundle.\n'
    return prefix+commentary+''.join(definitions)+''.join(clean),stats

def emit_tree(tree,versions,indent='    '):
    if tree['feature']<0:return indent+f'return {int(versions[tree["choice"]][1:])};\n'
    s=indent+f'if (f[{tree["feature"]}] <= {tree["threshold"].hex()}) {{\n'
    s+=emit_tree(tree['left'],versions,indent+'    ')+indent+'} else {\n'
    return s+emit_tree(tree['right'],versions,indent+'    ')+indent+'}\n'

def leaf_versions(tree,versions):
    if tree['feature']<0:return {versions[tree['choice']]}
    return leaf_versions(tree['left'],versions)|leaf_versions(tree['right'],versions)

def assemble(model):
    versions=sorted(leaf_versions(model['tree'],model['versions']))
    head='''// v320_instance_router.cpp
// v320: choose one frozen expert using a depth-at-most-two tree trained only
// on 320 fresh official-generator inputs. No seed or saved answer lookup.
// Full expert implementations and weights are retained below in namespaces.
// Selection, replay input parsing and every expert share this startup clock.
#include <bits/stdc++.h>
static const auto v320_process_start=std::chrono::steady_clock::now();
'''
    head+=FEATURE_CODE
    head+='static int v320_choose([[maybe_unused]] const std::array<double,17>& f) {\n'+emit_tree(model['tree'],model['versions'])+'}\n'
    chunks=[];changes={}
    for v in versions:
        raw=SOURCES[v].read_text();s=re.sub(r'^#include <bits/stdc\+\+\.h>\s*\n','',raw,flags=re.M)
        assert s.count('int main()')==1
        before,main=s.split('int main()',1)
        main,n=re.subn(r'time_keeper\.start_\s*=\s*chrono::steady_clock::now\(\);',
                       'time_keeper.start_ = ::v320_process_start;',main)
        assert n==1
        main=re.sub(r'\}\s*$','    return 0; // Implicit return is special to main; this is an engine function.\n}\n',main)
        changed=before+'int run_engine()'+main
        changes[v]=''.join(difflib.unified_diff(raw.splitlines(True),changed.splitlines(True),fromfile=SOURCES[v].name,tofile=v+'_namespaced_engine'))
        # Strip only full-line comments/blank lines and indentation. Strings,
        # arithmetic, loops and inline comments remain exactly as in the expert.
        changed='\n'.join(line.lstrip() for line in changed.splitlines() if line.strip() and not line.lstrip().startswith('//'))+'\n'
        block=f'\n// ===== frozen {v}: {sha(SOURCES[v].read_bytes())} =====\n'
        block+='#if defined(__GNUC__) && !defined(__clang__)\n#pragma GCC push_options\n#endif\n'
        block+=f'namespace engine_{v} {{\n'+changed+'}\n'
        block+='#undef LOCAL_NOTE\n#undef LOCAL_ONLY\n#undef LOCAL_TIME\n#undef LOCAL_SECONDS\n'
        block+='#if defined(__GNUC__) && !defined(__clang__)\n#pragma GCC pop_options\n#endif\n'
        chunks.append(block)
    tail='''
int main() {
    std::ios::sync_with_stdio(false);std::cin.tie(nullptr);
    int N,K;if(!(std::cin>>N>>K))return 1;
    if(N<12||N>20||K<4||K>12)return 1;
    std::vector<std::string> C(N);
    for(auto& row:C)if(!(std::cin>>row)||int(row.size())!=N)return 1;
    const auto feature=v320_features(N,K,C);
    const int choice=v320_choose(feature);
    std::string input=std::to_string(N)+" "+std::to_string(K)+"\\n";
    for(const auto& row:C)input+=row+"\\n";
    std::istringstream replay_input(input);
    auto* original=std::cin.rdbuf(replay_input.rdbuf());std::cin.clear();
    std::cerr<<"v320_choice="<<choice<<" v320_route_us="
      <<std::chrono::duration<double,std::micro>(std::chrono::steady_clock::now()-v320_process_start).count()<<'\\n';
    int result=1;
    switch(choice) {
'''
    for v in versions:tail+=f'        case {int(v[1:])}:engine_{v}::time_keeper.start_=v320_process_start;result=engine_{v}::run_engine();break;\n'
    tail+='''        default:throw std::logic_error("invalid frozen expert choice");
    }
    std::cin.rdbuf(original);return result;
}
'''
    return head+''.join(chunks)+tail,changes,versions

def main():
    for version,path in SOURCES.items():
        if sha(path.read_bytes())!=PARENTS[version]['sha256']:
            raise RuntimeError('Frozen parent drift: '+version)
    text,changes,versions=assemble(MODEL)
    final,stats=factor(text)
    if sha(final.encode())!=EXPECTED:raise RuntimeError('Frozen source mismatch')
    out=ROOT/'src/bin/v320_instance_router.cpp'
    if out.exists() and sha(out.read_bytes())!=EXPECTED:raise RuntimeError('Refusing to overwrite different v320')
    out.write_text(final)
    print(str(out),len(final.encode()),EXPECTED)
if __name__=='__main__':main()
