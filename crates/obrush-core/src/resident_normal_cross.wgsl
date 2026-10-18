fn raw_face(face:u32)->vec3<f32>{
 let ids=original_face(face);let a=position_rows[ids.x].xyz;
 let b=position_rows[ids.y].xyz;let c=position_rows[ids.z].xyz;let edge=c-a;
 var normal=cross(b-a,edge);
 if(ids.w!=0xffffffffu){normal+=cross(c-position_rows[ids.w].xyz,edge);}
 return normal;
}
