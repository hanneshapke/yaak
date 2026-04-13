# Cached result — instant replay of a previous query
# Prereq: run the command once without --cache first so the cache is warm
Output "07-cache.gif"
Set Width 800
Set Height 400
Set FontSize 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
# Warm the cache (hidden from recording)
Type "yaak convert video.mp4 to a gif at 10fps"
Enter
Sleep 5s
Type "n"
Enter
Sleep 1s
Type "clear"
Enter
Sleep 500ms
Show

Type "yaak --cache convert video.mp4 to a gif at 10fps"
Enter
Sleep 3s
