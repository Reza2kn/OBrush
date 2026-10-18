// Canonical triangles retain original face IDs and the explicit0-2 quad diagonal.
fn original_face(face:u32)->vec4<u32>{
 let header=face_headers[face];let at=(header>>1u)*3u;
 var d=0xffffffffu;if((header&1u)!=0u){d=original_indices[at+5u];}
 return vec4<u32>(original_indices[at],original_indices[at+1u],original_indices[at+2u],d);
}
fn face_selected(face:u32)->bool{
 let ids=original_face(face);
 if(selected[ids.x]!=0u||selected[ids.y]!=0u||selected[ids.z]!=0u){return true;}
 if(ids.w!=0xffffffffu){return selected[ids.w]!=0u;}return false;
}
