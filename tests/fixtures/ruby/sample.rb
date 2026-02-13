require "json"
require_relative "helpers"

MAX_RETRIES = 3

# A base animal class
class Animal
  KINGDOM = "Animalia"

  # Create a new animal
  def initialize(name, sound)
    @name = name
    @sound = sound
  end

  # Make the animal speak
  def speak
    puts "#{@name} says #{@sound}"
  end
end

# A dog inherits from Animal
class Dog < Animal
  def speak
    puts "#{@name} barks!"
  end

  def fetch(item)
    speak
    puts "Fetching #{item}"
  end

  def self.breed_info
    lookup_breeds
  end
end

# Utility module
module Formatter
  def self.format_name(name)
    name.strip.capitalize
  end

  def self.format_list(items)
    items.map { |i| format_name(i) }.join(", ")
  end
end

def greet(name)
  formatted = Formatter.format_name(name)
  puts "Hello, #{formatted}"
end
