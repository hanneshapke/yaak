# Onboarding wizard — interactive config setup
Output "01-wizard.gif"
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

Type "yaak --config"
Enter
Sleep 1s

# Language picker — pick english
Enter
Sleep 500ms

# Navigate to Ollama (option 3)
Down Down Down
Sleep 500ms
Enter
Sleep 1s

# Type model name
Down Down Down Down
Sleep 500ms
Enter
Sleep 500ms
Type "gemma4:e2b"
Enter
Sleep 1s

# Overwrite
Sleep 1s
Type "y"

Sleep 3s
