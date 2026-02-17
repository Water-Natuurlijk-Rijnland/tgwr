#!/bin/bash
# RLC Model Setup Script
# This downloads the required models for your RLC agent tier

echo "🤖 Setting up RLC Agent Models..."
echo ""

# Check if Ollama is installed
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama not found. Installing..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

echo "✅ Ollama installed"
echo ""
echo "📥 Downloading models for BALANCED tier..."
echo ""

ollama pull phi3.5:3.8b-instruct-q4_k_m
ollama pull mistral:7b-instruct-v0.3-q4_k_m
ollama pull llama3.2:3b-instruct-q4_k_m
ollama pull qwen2.5:7b-instruct-q4_k_m

echo ""
echo "✅ Model setup complete!"
echo ""
echo "Models downloaded:"
  - phi3.5:3.8b-instruct-q4_k_m
  - mistral:7b-instruct-v0.3-q4_k_m
  - llama3.2:3b-instruct-q4_k_m
  - qwen2.5:7b-instruct-q4_k_m

echo ""
echo "Resource requirements for this tier:"
echo "  VRAM: 16GB"
echo "  RAM: 32GB"
echo ""
echo "To start Ollama server:"
echo "  ollama serve"
echo ""
echo "To test a model:"
echo "  ollama run phi3.5:3.8b-instruct-q4_k_m"
