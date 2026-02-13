"""Sample Python module for testing."""

MAX_RETRIES: int = 3
DEFAULT_TIMEOUT = 30

def add(a: int, b: int) -> int:
    """Adds two numbers together."""
    return a + b

def greet(name: str) -> str:
    result = format_name(name)
    return f"Hello, {result}"

class Animal:
    """Base animal class."""
    MAX_AGE = 100

    def __init__(self, name: str, sound: str) -> None:
        self.name = name
        self.sound = sound

    def speak(self) -> str:
        """Make the animal speak."""
        return f"{self.name} says {self.sound}"

class Dog(Animal):
    """A dog that inherits from Animal."""

    def speak(self) -> str:
        return f"{self.name} barks!"

    def fetch(self, item: str) -> str:
        result = self.speak()
        print(result)
        return f"Fetching {item}"

@staticmethod
def standalone_decorated() -> None:
    pass
