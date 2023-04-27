
# Auto

A command line utility that exectues commands when a file gets modified

# Why?

- I wanted a tool that can do what cargo watch does but for C/C++ projects(See example 1)

# Usage
Pipe some files/directories to watch into auto execute a command when one of those files are modified
'''bash
 echo "text.txt" | auto "cat text.txt"
'''

# Examples

## Example 1

- automatically rebuild the project in the background whenever I change any source file

'''bash
 find ./src -name '*.cpp' | auto "make"
'''

- you could produce the same effect with the -r flag which watches directories recursively
'''bash
 echo "src" | auto -r "make"
'''
