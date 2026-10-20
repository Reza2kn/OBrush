// Independent column-major projection/ray oracle for the authored two-face OBJ.
const vertices=[[-1.2,-0.8,0],[1,-0.8,0],[1,.9,.15],[-1.2,.9,.4]];
const faces=[[0,1,2],[0,2,3]];
const mul=(m,p)=>[0,1,2,3].map(i=>m[i]*p[0]+m[i+4]*p[1]+m[i+8]*p[2]+m[i+12]*p[3]);
const product=(a,b)=>[0,1,2,3].flatMap(j=>[0,1,2,3].map(i=>[0,1,2,3].reduce((s,k)=>s+a[i+4*k]*b[k+4*j],0)));
function inverse(m){
  const a=Array.from({length:4},(_,r)=>[...Array.from({length:4},(_,c)=>m[r+4*c]),...Array.from({length:4},(_,c)=>Number(r===c))]);
  for(let k=0;k<4;k++){let row=k;for(let r=k+1;r<4;r++)if(Math.abs(a[r][k])>Math.abs(a[row][k]))row=r;
    if(Math.abs(a[row][k])<1e-12)throw Error('Singular camera matrix');[a[k],a[row]]=[a[row],a[k]];
    const pivot=a[k][k];for(let c=0;c<8;c++)a[k][c]/=pivot;
    for(let r=0;r<4;r++)if(r!==k){const f=a[r][k];for(let c=0;c<8;c++)a[r][c]-=f*a[k][c];}}
  return Array.from({length:16},(_,n)=>a[n%4][4+Math.floor(n/4)]);
}
const sub=(a,b)=>a.map((v,i)=>v-b[i]);
const dot=(a,b)=>a.reduce((s,v,i)=>s+v*b[i],0);
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
export function projectWorld(view,projection,point,screen){
  const p=mul(product(projection,view),[...point,1]);
  return [screen[0]+(p[0]/p[3]+1)*(screen[2]-screen[0])/2,screen[1]+(1-p[1]/p[3])*(screen[3]-screen[1])/2];
}
export function integerPixelRayHit(view,projection,pixel,screen){
  const inv=inverse(product(projection,view));
  const ndc=[2*(pixel[0]-screen[0])/(screen[2]-screen[0])-1,1-2*(pixel[1]-screen[1])/(screen[3]-screen[1])];
  const unproject=z=>{const p=mul(inv,[...ndc,z,1]);return p.slice(0,3).map(v=>v/p[3]);};
  // SculptGL uses an infinite far plane, so NDC z=1 has homogeneous w=0.
  const origin=unproject(-1),end=unproject(.5),direction=sub(end,origin);let best=null;
  for(const [i,j,k] of faces){const a=vertices[i],b=vertices[j],c=vertices[k],edge1=sub(b,a),edge2=sub(c,a),p=cross(direction,edge2),det=dot(edge1,p);if(Math.abs(det)<1e-12)continue;
    const invdet=1/det,tvec=sub(origin,a),u=dot(tvec,p)*invdet;if(u<0||u>1)continue;const q=cross(tvec,edge1),v=dot(direction,q)*invdet;if(v<0||u+v>1)continue;
    const t=dot(edge2,q)*invdet;if(t<0||best&&t>=best.t)continue;best={t,face:[i,j,k],barycentric:[1-u-v,u,v],point:origin.map((x,n)=>x+direction[n]*t)};}
  if(!best)throw Error('Frozen integer pick missed authored mesh');return best;
}
