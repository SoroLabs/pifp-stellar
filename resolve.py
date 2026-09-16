with open("contracts/pifp_protocol/src/test.rs", "r") as f:
    lines = f.readlines()

out = []
skip = False
for line in lines:
    if line.startswith("<<<<<<< HEAD"):
        skip = True
    elif line.startswith("======="):
        skip = False
    elif line.startswith(">>>>>>> origin/pr-38"):
        pass
    else:
        if not skip:
            out.append(line)

with open("contracts/pifp_protocol/src/test.rs", "w") as f:
    f.writelines(out)
