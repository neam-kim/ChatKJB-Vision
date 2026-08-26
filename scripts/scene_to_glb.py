#!/usr/bin/env python3
"""Thin adapter around trimesh; mesh topology is provided by the library."""
import json, sys
import numpy as np
import trimesh

def c(x):
    x=x.lstrip('#'); x = x if len(x)==6 else {'red':'ff0000','blue':'0000ff','yellow':'ffff00','green':'00ff00'}.get(x,'aaaaaa')
    return [int(x[i:i+2],16) for i in (0,2,4)] + [255]
def main(src,out):
    d=json.load(open(src)); parts=[]
    for o in d.get('objects',[]):
        typ=o['type']; s=o.get('scale',[1,1,1])
        if typ=='sphere': m=trimesh.creation.icosphere(subdivisions=2)
        elif typ=='cube': m=trimesh.creation.box(extents=[2,2,2])
        elif typ=='cylinder': m=trimesh.creation.cylinder(radius=1,height=2)
        elif typ=='cone': m=trimesh.creation.cone(radius=1,height=2)
        else: continue
        m.apply_scale(s); m.apply_translation(o.get('position',[0,0,0])); m.visual.face_colors=np.tile(c(o.get('color','aaaaaa')),(len(m.faces),1)); parts.append(m)
    for a in d.get('arrows',[]):
        v=np.array(a['to'])-np.array(a['from']); L=np.linalg.norm(v)
        if L: m=trimesh.creation.cylinder(radius=.06,height=L); m.apply_translation((np.array(a['from'])+np.array(a['to']))/2); m.visual.face_colors=np.tile(c(a.get('color','yellow')),(len(m.faces),1)); parts.append(m)
    trimesh.util.concatenate(parts or [trimesh.creation.box()]).export(out,file_type='glb')
if __name__=='__main__': main(sys.argv[1],sys.argv[2])
