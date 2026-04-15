# Kill a process on port 3000
# Prereq: run `demos/helpers/fake-server.sh start` before recording
Output "02-kill-port.gif"
Set Width 800
Set Height 400
Set FontSize 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
Type "clear"
Enter
Sleep 500ms
Show

Type "yaak find what\'s using port 3000"
Enter
Sleep 2s

# Confirm execution
Type "y"
Sleep 500ms
Enter
Sleep 500ms
Hide
Type "..."
Enter
Show
Sleep 2s
