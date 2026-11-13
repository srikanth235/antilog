import sys,re
t=open(sys.argv[1]).read()
t=re.sub(r'(?m)^[ \t]+','',t)
open(sys.argv[2],'w').write(t)
