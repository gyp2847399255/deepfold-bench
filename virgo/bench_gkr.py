import os
import random, string

def randomword(length):
   letters = string.ascii_lowercase
   return ''.join(random.choice(letters) for i in range(length))

tmp_file = randomword(20) + '.txt'
for i in range(5, 20):
    print('bench fft_gkr of size', i)
    round = 10
    prover_time = 0
    verifier_time = 0
    proof_size = 0
    for j in range(0, round):
        os.system('./fft_gkr ' + str(i) + ' ' + tmp_file)
        f = open(tmp_file)
        lines = f.readlines()
        v, ps, p = lines[0].split(' ')
        prover_time += float(p)
        proof_size += float(ps)
        verifier_time += float(v)
    print('prover time:', prover_time / round, 's')
    print('verifier time:', verifier_time / round, 's')
    print('proof size:', proof_size / round, 'bytes')
os.remove(tmp_file)