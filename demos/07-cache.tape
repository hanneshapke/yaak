# Cached result — instant replay of a previous query
# Prereq: run the command once without --cache first so the cache is warm
Output "07-cache.gif"
Set Width 800
Set Height 400
Set FontSize 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

# Warm the cache (hidden from recording)
Type "yaak get the weather forecasts for San Francisco, Tokyo, and London. save as json."
Enter
Sleep 15s
Down Down Down
Enter
Sleep 1s
Type "clear"
Enter
Sleep 500ms
Show

Type "yaak --cache get the weather forecasts for San Francisco, Tokyo, and London. save as json."
Enter
Sleep 3s
