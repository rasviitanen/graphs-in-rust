import subprocess
import sys
from shutil import copyfile


def run(cmd):
    proc = subprocess.Popen(cmd,
        stdout = subprocess.PIPE,
        stderr = subprocess.PIPE,
    )
    stdout, stderr = proc.communicate()

    return proc.returncode, stdout, stderr

take = 0;
def element_selector(element):
    global take;
    if take > 0:
        take -= 1;
        return 1;
    if element.decode('utf-8') in ['Analyzing']:
        take = 1;
        return 0
    if element.decode('utf-8')[0:2] in ['[+', '[-']:
        take = 2;
        return 1
    if element.decode('utf-8')[0] in ['[']:
        take = 5;
        return 1
    if element.decode('utf-8') in ['(p']:
        take = 4;
        return 1
    if element.decode('utf-8') in ['Performance']:
        take = 2;
        return 1
    if element.decode('utf-8') in ['No']:
        take = 3;
        return 1
    return 0


def run_until_no_change(kernel):
    print("[" + kernel + "]" + "...starting first run...")
    code, out, err = run(["cargo", "bench", "--features", kernel])
    print("First run completed, starting second...")
    code, out, err = run(["cargo", "bench", "--features", kernel])
    test_results = list(filter(element_selector, out.split()))
    while (b'improved.' in test_results) or (b'regressed.' in test_results):
        print(test_results);
        print("Change Detected, running again...");
        code, out, err = run(["cargo", "bench", "--features", kernel])
        test_results = list(filter(element_selector, out.split()))
        for word in test_results:
            if b'/' in word:
                print('\n')
            print(word.decode('utf-8') + ' ', end='')

    print("Everything is OK! Wrtiing test report...")
    filename = "reports/euroroad/" + kernel + ".txt"
    myfile = open(filename, 'wb')
    for word in test_results:
        if b'/' in word:
            myfile.write(b'\n')
        myfile.write(word + b' ')
    myfile.close()

    print("Copyting violin plot")
    copyfile("target/criterion/" + kernel + "/report/violin.svg", "reports/euroroad/" + kernel + ".svg")
    print("[" + kernel + "]" + " DONE!")


# run_until_no_change("bc");
run_until_no_change("bfs");
run_until_no_change("tc");
run_until_no_change("sssp");
run_until_no_change("cc");
run_until_no_change("pr");