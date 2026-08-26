#!/usr/bin/env python3
"""Thin adapter around trimesh; mesh topology is provided by the library."""
import json
import sys
import numpy as np
import trimesh

def c(x):
    names={'red':'ff0000','blue':'0000ff','yellow':'ffff00','green':'00ff00'}
    x=names.get(x.lower(),x.lstrip('#'))
    if len(x)!=6 or any(ch not in '0123456789abcdefABCDEF' for ch in x):
        raise ValueError(f"invalid color: {x}")
    return [int(x[i:i+2],16) for i in (0,2,4)]
def transform(m, o):
    m.apply_scale(o.get('scale',[1,1,1]))
    r=np.array(o.get('rotation',[0,0,0]),dtype=float)
    if np.any(r):
        # API rotations are degrees XYZ.
        for axis, angle in zip(np.eye(3), r):
            if angle: m.apply_transform(trimesh.transformations.rotation_matrix(np.deg2rad(angle),axis))
    m.apply_translation(o.get('position',[0,0,0]))
    rgba=c(o.get('color','aaaaaa'))+[round(255*float(o.get('opacity',1)))]
    m.visual.face_colors=np.tile(rgba,(len(m.faces),1)); return m
def segment(a,b,radius,color):
    a=np.array(a,dtype=float); b=np.array(b,dtype=float); v=b-a; L=np.linalg.norm(v)
    if not L: return None
    z=np.array([0.,0.,1.]); axis=np.cross(z,v/L); dot=np.dot(z,v/L)
    m=trimesh.creation.cylinder(radius=radius,height=L)
    if np.linalg.norm(axis)>1e-8: m.apply_transform(trimesh.transformations.rotation_matrix(np.arccos(np.clip(dot,-1,1)),axis))
    elif dot<0: m.apply_transform(trimesh.transformations.rotation_matrix(np.pi,[1,0,0]))
    m.apply_translation((a+b)/2); m.visual.face_colors=np.tile(color,(len(m.faces),1)); return m

def arrow(a, b, radius, color):
    a=np.array(a,dtype=float); b=np.array(b,dtype=float); v=b-a; length=np.linalg.norm(v)
    if not length: return None
    head_length=min(max(radius*5.0, .18), length*.45)
    unit=v/length
    shaft=segment(a, b-unit*head_length, radius, color)
    head=trimesh.creation.cone(radius=radius*2.4,height=head_length,sections=16)
    z=np.array([0.,0.,1.]); axis=np.cross(z,unit); dot=np.dot(z,unit)
    if np.linalg.norm(axis)>1e-8: head.apply_transform(trimesh.transformations.rotation_matrix(np.arccos(np.clip(dot,-1,1)),axis))
    elif dot<0: head.apply_transform(trimesh.transformations.rotation_matrix(np.pi,[1,0,0]))
    head.apply_translation(b-unit*head_length/2)
    head.visual.face_colors=np.tile(color,(len(head.faces),1))
    return trimesh.util.concatenate([shaft,head])
def main(src,out):
    d=json.load(open(src)); parts=[]
    for o in d.get('objects',[]):
        typ=o['type']
        if typ=='sphere': m=trimesh.creation.icosphere(subdivisions=2)
        elif typ=='cube': m=trimesh.creation.box(extents=[2,2,2])
        elif typ=='cylinder': m=trimesh.creation.cylinder(radius=1,height=2)
        elif typ=='cone': m=trimesh.creation.cone(radius=1,height=2)
        elif typ in ('line','arrow'):
            col=c(o.get('color','aaaaaa'))+[round(255*float(o.get('opacity',1)))]
            factory=arrow if typ=='arrow' else segment
            m=factory(o.get('from',o.get('start',[0,0,0])),o.get('to',o.get('end',[0,0,1])),.06 if typ=='arrow' else .04,col)
            if m is not None: parts.append(transform(m,o))
            continue
        else: continue
        parts.append(transform(m,o))
    for a in d.get('arrows',[]):
        col=c(a.get('color','yellow'))+[round(255*float(a.get('opacity',1)))]
        m=arrow(a['from'],a['to'],.06,col)
        if m is not None: parts.append(m)
    if not parts:
        raise ValueError("scene contains no supported geometry")
    trimesh.util.concatenate(parts).export(out,file_type='glb')
if __name__=='__main__': main(sys.argv[1],sys.argv[2])
