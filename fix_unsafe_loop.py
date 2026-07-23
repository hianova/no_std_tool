import json
import sys
import os
import subprocess

def run_fix():
    print("Running cargo check...")
    res = subprocess.run(["cargo", "check", "--message-format=json"], capture_output=True, text=True)
    
    file_errors = {}
    error_count = 0
    for line in res.stdout.split('\n'):
        if not line: continue
        try:
            msg = json.loads(line)
        except:
            continue
        if msg.get('reason') != 'compiler-message':
            continue
        message = msg.get('message', {})
        code_info = message.get('code') or {}
        if code_info.get('code') != 'E0133':
            continue
        
        spans = message.get('spans', [])
        primary_spans = [s for s in spans if s.get('is_primary')]
        if not primary_spans:
            continue
        
        span = primary_spans[0]
        filename = span.get('file_name')
        if not filename:
            continue
        if 'vec101_compute/simd' not in filename:
            continue
            
        if filename not in file_errors:
            file_errors[filename] = []
            
        file_errors[filename].append({
            'line_start': span['line_start'],
            'line_end': span['line_end'],
            'column_start': span['column_start'],
            'column_end': span['column_end'],
        })
        error_count += 1

    if error_count == 0:
        return False
        
    for filename, errors in file_errors.items():
        if not os.path.exists(filename):
            print(f"File not found: {filename}")
            continue
            
        with open(filename, 'r') as f:
            lines = f.read().split('\n')
            
        # Sort errors descending by line, then descending by column
        errors.sort(key=lambda x: (x['line_start'], x['column_start']), reverse=True)
        
        for e in errors:
            l_start = e['line_start'] - 1
            l_end = e['line_end'] - 1
            c_start = e['column_start'] - 1
            c_end = e['column_end'] - 1
            
            if l_start == l_end:
                line = lines[l_start]
                prefix = line[:c_start]
                target = line[c_start:c_end]
                suffix = line[c_end:]
                if "unsafe {" not in target:
                    lines[l_start] = f"{prefix}unsafe {{ {target} }}{suffix}"
            else:
                if "unsafe {" not in lines[l_start][c_start:]:
                    lines[l_start] = lines[l_start][:c_start] + "unsafe { " + lines[l_start][c_start:]
                    lines[l_end] = lines[l_end][:c_end] + " }" + lines[l_end][c_end:]
                
        with open(filename, 'w') as f:
            f.write('\n'.join(lines))
            
    print(f"Fixed {error_count} errors")
    return True

for i in range(10):
    if not run_fix():
        print("Done!")
        break
