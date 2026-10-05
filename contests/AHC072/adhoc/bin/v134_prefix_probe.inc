struct PrefixProbeTimer { bool step()const{return true;} };
int main() {
    nn::Input original;original.read();nn::load_nn();
    for(int sym:{0,4,1,5}) {
        const auto input=neural_transform_input(original,sym);
        const nn::Board board(input);const nn::BoardState initial(input,board);
#ifdef V134_PARENT
        const int modes=1;
#else
        const int modes=(sym==1 || sym==5)?2:1;
#endif
        for(int diverse=0;diverse<modes;++diverse) {
            PrefixProbeTimer timer;
#ifdef V134_PARENT
            const auto result=nn::search(input,board,initial,timer);
            const int steps=0,reorders=0,last=0;
#else
            const auto result=nn::search(input,board,initial,timer,diverse);
            const int steps=result.prefix_steps,reorders=result.prefix_reorders,last=result.last_prefix_depth;
#endif
            cout<<"START "<<sym<<' '<<diverse<<' '<<result.path.size()<<' '<<result.E<<' '
                <<steps<<' '<<reorders<<' '<<last<<'\n';
            for(nn::Move m:result.path) {
                const auto [i,j]=board.coordinates[m.p()];
                const auto [qi,qj]=board.coordinates[board.ray[m.p()][m.d()][m.l()-1]];
                const auto [r,c]=neural_transform_cell(i,j,input.N,sym,true);
                const auto [qr,qc]=neural_transform_cell(qi,qj,input.N,sym,true);
                const int direction=qr<r?0:qr>r?1:qc<c?2:3;
                cout<<r<<' '<<c<<' '<<m.k()<<' '<<"UDLR"[direction]<<' '<<m.l()<<'\n';
            }
        }
    }
}
