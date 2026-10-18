fn selected_mean(vertex:u32)->vec3<f32>{
 let start=vertex_offsets[vertex];let end=vertex_offsets[vertex+1u];var sum=vec3<f32>(0.);
 // Ring rows are in descending original face order; one invocation owns each mean.
 for(var at=start;at<end;at++){let f=incident_faces[at]*3u;sum+=vec3<f32>(face_rows[f],face_rows[f+1u],face_rows[f+2u]);}
 if(end==start){return vec3<f32>(0.);}return sum*(1./f32(end-start));
}
