import torch
import torch.nn as nn
import torch.nn.functional as F

class MarbleNeuralNetwork(nn.Module):
    def __init__(self):
        super().__init__()
        self.fc1 = nn.Linear(10, 10)     # 9 inputs -> 10 hidden neurons
        self.fc2 = nn.Linear(10, 5)     # 10 -> 5 outputs

    def forward(self, x):
        x = F.relu(self.fc1(x))
        x = torch.sigmoid(self.fc2(x))  # Output values between 0 and 1
        return x