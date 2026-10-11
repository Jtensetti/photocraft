// webm-muxer 5.1.4 stores the last packet timestamp as Segment Duration,
// omitting the final frame duration. Correct only that eight-byte header field.
// Walk actual EBML elements, never scan for a byte pattern in codec payloads.
export function durationOffset(bytes){
  let elements=0;
  function integer(at,id=false){
    if(at>=bytes.length||!bytes[at])throw new Error('Ogiltigt WebM-huvud');
    let marker=128,length=1;while(!(bytes[at]&marker)){marker>>=1;length++;}
    if(length>(id?4:8)||at+length>bytes.length)throw new Error('Ofullständigt WebM-huvud');
    let value=BigInt(id?bytes[at]:bytes[at]&(marker-1));for(let i=1;i<length;i++)value=value*256n+BigInt(bytes[at+i]);
    if(value>BigInt(Number.MAX_SAFE_INTEGER))throw new Error('För stort WebM-element');
    return {value:Number(value),length};
  }
  function walk(start,end,depth){
    if(depth>3)throw new Error('För djupt WebM-huvud');
    for(let at=start;at<end;){
      if(++elements>1024)throw new Error('För många WebM-huvudelement');
      const id=integer(at,true),size=integer(at+id.length),body=at+id.length+size.length,next=body+size.value;
      if(next>end||next<=at)throw new Error('Ogiltig WebM-elementstorlek');
      if(id.value===0x4489){if(size.value!==8)throw new Error('WebM-varaktigheten måste vara flyttal med åtta byte');return body;}
      if(id.value===0x18538067||id.value===0x1549a966){const found=walk(body,next,depth+1);if(found!==null)return found;}
      at=next;
    }
    return null;
  }
  return walk(0,bytes.length,0);
}
export function durationBytes(milliseconds){const bytes=new Uint8Array(8);new DataView(bytes.buffer).setFloat64(0,milliseconds,false);return bytes;}
